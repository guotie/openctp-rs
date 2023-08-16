## mdapi.rs

on macos(M1/M2/etc):
```
export DYLD_FALLBACK_LIBRARY_PATH=$DYLD_FALLBACK_LIBRARY_PATH:`pwd`/shared/macos.arm64
```

```
pip3 install libclang
```

on Linux(x64):
```
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:`pwd`/shared/linux.x64
```
