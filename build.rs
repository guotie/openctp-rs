use std::env;
use std::path::{Path, PathBuf};
use regex::Regex;

fn main() {
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "arm") || cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        panic!("not support this arch")
    };
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        panic!("not support this os")
    };
    let cur = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=src/wrapper.hpp");
    println!("cargo:rerun-if-changed=src/wrapper.cpp");
    println!("cargo:rerun-if-changed=src/wrapper_ext.hpp");

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", cur.join("shared").join(format!("{}.{}", os, arch)).display());
    // Tell cargo to tell rustc to link the shared library.
    println!("cargo:rustc-link-lib=dylib=thostmduserapi_se");
    println!("cargo:rustc-link-lib=dylib=thosttraderapi_se");
    // if use this, dyld[43510]: Library not loaded: thostmduserapi_se.dylib
    // println!("cargo:rustc-link-arg={}", cur.join("shared").join(format!("{}.{}/thostmduserapi_se.dylib", os, arch)).display());
    // println!("cargo:rustc-link-arg={}", cur.join("shared").join(format!("{}.{}/thosttraderapi_se.dylib", os, arch)).display());

    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-w")
        .file("src/wrapper.cpp")
        .compile("wrapper");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        // .header("src/wrapper.hpp")
        .header("src/wrapper_ext.hpp")
        // .derive_debug(false).layout_tests(false).generate_comments(false)
        // Generate vtable
        .vtable_generation(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .opaque_type("CThostFtdcTraderApi")
        .opaque_type("CThostFtdcTraderSpi")
        .opaque_type("CThostFtdcMdApi")
        .opaque_type("CThostFtdcMdSpi")
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_file = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    let buf = bindings.to_string();
        // .write_to_file(out_path.join("bindings.rs"))
        // .expect("Couldn't write bindings!");
    let buf2 = replace_trait(buf, &["Rust_CThostFtdcMdSpi_Trait", "Rust_CThostFtdcTraderSpi_Trait"]).
        expect("Fail to replace trait!");
    std::fs::write(&out_file, &buf2)
        .expect("Fail to write converted bindings!");
}

fn camel_to_snake(name: &str) -> String {
    let camel: Regex = Regex::new(r"(.)([A-Z][a-z]+)").unwrap();
    let snake: Regex = Regex::new(r"([a-z0-9])([A-Z])").unwrap();
    snake.replace_all(camel.replace_all(name, r"${1}_${2}").as_ref(), r"${1}_${2}").to_lowercase()
}


fn replace_trait(mut buf: String, traits: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    // let mut buf = std::fs::read_to_string(fname)?;
    for trait_extern in traits {
        let pattern = Regex::new(
            &format!(r#"extern \s*"C"\s*\{{\s*pub\s+fn\s+{}_(\w+)\s*\(([^)]*)\)([^;]*);\s*}}\s*"#, trait_extern)).unwrap();
        let pattern_arg = Regex::new(r"\s*(\w+)\s*:\s*(.*)\s*").unwrap();

        let mut exports = vec![];
        let mut traitfuns = vec![];
        assert!(pattern.captures(&buf).is_some(), "`{}` not found in source code", trait_extern);
        for cap in pattern.captures_iter(&buf) {
            let fname = cap.get(1).unwrap().as_str().trim();
            let args: Vec<_> = cap.get(2).unwrap().as_str().split(',').filter(
                |s| !s.trim().is_empty()
            ).map(
                |s| { let c = pattern_arg.captures(s).unwrap(); (c.get(1).unwrap().as_str(), c.get(2).unwrap().as_str()) }
            ).collect();
            let rtn = cap.get(3).unwrap().as_str();
            let fname_camel = camel_to_snake(fname);
            if fname_camel == "drop" { continue }
            assert!(args[0].1.trim().ends_with("c_void"));

            let mut tmp = args[1..].iter().map(|s| format!("{}: {}", s.0, s.1)).collect::<Vec<_>>();
            tmp.insert(0, "trait_obj: *mut ::std::os::raw::c_void".into());
            let args_repl = tmp.join(", ");
            let argv_repl = args[1..].iter().map(|s| s.0).collect::<Vec<_>>().join(", ");

            let export = format!(r#"#[no_mangle]
pub extern "C" fn {trait_extern}_{fname}({args_repl}){rtn} {{
    let trait_obj = trait_obj as *mut Box<dyn {trait_extern}>;
    let trait_obj: &mut dyn {trait_extern} = unsafe {{ &mut **trait_obj }};
    trait_obj.{fname_camel}({argv_repl})
}}
"#, trait_extern=trait_extern, fname=fname, args_repl=args_repl, rtn=rtn, fname_camel=fname_camel, argv_repl=argv_repl);
            exports.push(export);

            let mut tmp = args[1..].iter().map(|s| format!("{}: {}", s.0, s.1)).collect::<Vec<_>>();
            tmp.insert(0, "&mut self".into());
            let args_repl = tmp.join(", ");
            let traitfun = format!(r"    fn {fname_camel}({args_repl}){rtn} {{  }}", fname_camel=fname_camel, args_repl=args_repl, rtn=rtn );
            traitfuns.push(traitfun);
        }

        let exports_repl = exports.join("\n");
        let traitfuns_repl = traitfuns.join("\n");

        buf = format!(r#"{ori}
#[allow(unused)]
pub trait {trait_extern} {{
{traitfuns_repl}
}}

{exports_repl}
#[no_mangle]
pub extern "C" fn {trait_extern}_Drop(trait_obj: *mut ::std::os::raw::c_void) {{
    let trait_obj = trait_obj as *mut Box<dyn {trait_extern}>;
    let _r: Box<Box<dyn {trait_extern}>> = unsafe {{ Box::from_raw(trait_obj) }};
}}
"#, ori = pattern.replace_all(&buf, ""), exports_repl=exports_repl, trait_extern=trait_extern,
traitfuns_repl=traitfuns_repl
            );
    }

    Ok(buf)
}
