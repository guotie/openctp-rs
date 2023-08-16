#![allow(non_snake_case)]
#![allow(dead_code)]
use std::os::raw::*;
use std::ffi::{CStr, CString};
use std::sync::mpsc::{self, SyncSender, Receiver};

use openctp_rs::{CThostFtdcReqUserLoginField,
    CThostFtdcRspUserLoginField,
    CThostFtdcRspInfoField,
    CThostFtdcSpecificInstrumentField,
    CThostFtdcDepthMarketDataField};
use openctp_rs::{CThostFtdcMdApi, Rust_CThostFtdcMdApi, Rust_CThostFtdcMdSpi, Rust_CThostFtdcMdSpi_Trait};


// openctp: http://121.37.80.177:50080/detail.html

#[derive(Debug, Clone)]
pub enum Event {
    Connected,
    UserLogin,
    Disconnected(i32),

    Unhandled(String),
}

struct CTPMDSpi {
    events: SyncSender<Event>
}

pub struct CTPMDApi {
    api: Rust_CThostFtdcMdApi,
    spi: Option<*mut Rust_CThostFtdcMdSpi>,
    rx: Option<Receiver<Event>>,

    // config
    flowpath: String,
    front_addr: String,
    nm_addr: String,
    is_udp: bool,
    is_multicast: bool,
}

impl Rust_CThostFtdcMdSpi_Trait for CTPMDSpi {
	// 当客户端与交易后台建立起通信连接时（还未登录前），该方法被调用。
    fn on_front_connected(&mut self) {
        println!("front connected");
        self.events.send(Event::Connected).unwrap();
    }

    fn on_front_disconnected(&mut self, reason: ::std::os::raw::c_int) {
        println!("front disconnected, reason: {}", reason);
        self.events.send(Event::Disconnected(reason)).unwrap();
    }

    // 登录请求响应
	fn on_rsp_user_login(&mut self,
        _pRspUserLogin: *mut CThostFtdcRspUserLoginField,
        _pRspInfo: *mut CThostFtdcRspInfoField,
        nRequestID: ::std::os::raw::c_int,
        _bIsLast: bool) {
        println!("user login, requsetID: {}", nRequestID);
        self.events.send(Event::UserLogin).unwrap()
    }

	// 深度行情通知
	fn on_rtn_depth_market_data(&mut self, pDepthMarketData: *mut CThostFtdcDepthMarketDataField) {
        if pDepthMarketData.is_null() {
            println!("empty data");
            return
        }
        unsafe {
            let data = *pDepthMarketData.clone();
            println!("depth: {} {}", data.AskPrice1, data.BidPrice1)
        }
    }

	// 登出请求响应
	// fn OnRspUserLogout(&mut self, CThostFtdcUserLogoutField *pUserLogout, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 请求查询组播合约响应
	// fn OnRspQryMulticastInstrument(&mut self, CThostFtdcMulticastInstrumentField *pMulticastInstrument, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 错误应答
	// fn OnRspError(&mut self, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 订阅行情应答
    fn on_rsp_sub_market_data(&mut self,
        _pSpecificInstrument: *mut CThostFtdcSpecificInstrumentField,
        _pRspInfo: *mut CThostFtdcRspInfoField,
        nRequestID: ::std::os::raw::c_int,
        _bIsLast: bool) {
        print!("sub md: {}", nRequestID)
    }

	// 取消订阅行情应答
	// fn OnRspUnSubMarketData(&mut self, CThostFtdcSpecificInstrumentField *pSpecificInstrument, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 订阅询价应答
	// fn OnRspSubForQuoteRsp(&mut self, CThostFtdcSpecificInstrumentField *pSpecificInstrument, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 取消订阅询价应答
	// fn OnRspUnSubForQuoteRsp(&mut self, CThostFtdcSpecificInstrumentField *pSpecificInstrument, CThostFtdcRspInfoField *pRspInfo, int nRequestID, bool bIsLast) {};

	// 询价通知
	// fn OnRtnForQuoteRsp(CThostFtdcForQuoteRspField *pForQuoteRsp) {};
}

impl CTPMDApi {
    pub fn get_version() -> String {
        let cs = unsafe { CStr::from_ptr(CThostFtdcMdApi::GetApiVersion()) };
        cs.to_string_lossy().into()
    }

    pub fn new(flowpath: String, front_addr: String, nm_addr: String, is_udp: bool, is_multicast: bool) -> Self {
        let cs = std::ffi::CString::new(flowpath.as_bytes()).unwrap();
        let api = unsafe {
            Rust_CThostFtdcMdApi::new(CThostFtdcMdApi::CreateFtdcMdApi(cs.as_ptr(), is_udp, is_multicast))
        };
        Self { api, spi: None, rx: None, flowpath, front_addr, nm_addr, is_udp, is_multicast }
    }

    fn register<S: Rust_CThostFtdcMdSpi_Trait>(&mut self, spi: S) {
        if let Some(spi) = self.spi.take() {
            println!("des old registered spi");
            Self::drop_spi(spi);
        }

        let spi: Box<Box<dyn Rust_CThostFtdcMdSpi_Trait>> = Box::new(Box::new(spi));
        let ptr = Box::into_raw(spi) as *mut _ as *mut c_void;

        let spi_stub = unsafe { Rust_CThostFtdcMdSpi::new(ptr) } ;
        let spi: *mut Rust_CThostFtdcMdSpi = Box::into_raw(Box::new(spi_stub));
        unsafe { self.api.RegisterSpi(spi as _); }

        self.spi = Some(spi);
    }

    fn drop_spi(spi: *mut Rust_CThostFtdcMdSpi) {
        let mut spi = unsafe { Box::from_raw(spi) };
        unsafe { spi.destruct(); }
    }

    fn req_user_login(&mut self) -> Result<(), String> {
        // let loginfield : CThostFtdcReqUserLoginField = todo!();
        let mut loginfield = CThostFtdcReqUserLoginField {
            TradingDay:           Default::default(),
            BrokerID:             Default::default(),
            UserID:               Default::default(),
            Password:             [0i8; 41],
            UserProductInfo:      Default::default(),
            InterfaceProductInfo: Default::default(),
            ProtocolInfo:         Default::default(),
            MacAddress:           Default::default(),
            OneTimePassword:      [0i8; 41],
            reserve1:             Default::default(),
            ClientIPAddress:      [0i8; 33], // Default::default(),
            LoginRemark:          [0i8; 36],
            ClientIPPort:         Default::default(),
        };

        unsafe { self.api.ReqUserLogin(&mut loginfield, 1); }
        Ok(())
    }

    fn req_init(&mut self) -> Result<(), String> {
        let (tx, rx) = mpsc::sync_channel(1024);
        self.register(CTPMDSpi { events: tx });
        self.rx = Some(rx);
        println!("start api...");

        if self.front_addr.len() > 0 {
            println!("front_addr is: {}", self.front_addr);
            let cs = CString::new(self.front_addr.as_bytes()).unwrap();
            unsafe { self.api.RegisterFront(cs.as_ptr() as *mut _); }
        }

        if self.nm_addr.len() > 0 {
            println!("nm_addr is: {}", self.front_addr);
            let cs = CString::new(self.nm_addr.as_bytes()).unwrap();
            unsafe { self.api.RegisterNameServer(cs.as_ptr() as *mut _); }
        }

        unsafe { self.api.Init(); }

        Ok(())
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.req_init()?;
        assert!(self.rx.is_some(), "channel not started.");

        let rx = self.rx.as_mut().unwrap();
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Err(_) => { 
                    return Err("Timeout try recv `req_init`".into())
            }
            Ok(Event::Connected) => { }
            Ok(event) => {
                return Err(format!("invalid event: {:?}", event))
            }
        }

        self.req_user_login()?;

        let rx = self.rx.as_mut().unwrap();
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Err(_) => { 
                return Err("Timeout try recv `req_user_login`".into())
            }
            Ok(Event::UserLogin) => { }
            Ok(event) => {
                return Err(format!("invalid event: {:?}", event))
            }
        }

        Ok(())
    }

    pub fn subscribe_market_data(&mut self, codes: &[&str], is_unsub: bool) -> Result<(), String> {
        let len = codes.len() as c_int;
        let arr_cstring: Vec<CString> = codes.iter().map(|s| CString::new(s.as_bytes()).unwrap()).collect();
        let arr_cstr: Vec<*mut c_char> = arr_cstring.iter().map(|s| s.as_ptr() as *mut c_char).collect();
        let ptr = arr_cstr.as_ptr() as *mut *mut c_char;
        let rtn = if is_unsub {
            unsafe { self.api.UnSubscribeMarketData(ptr, len) }
        } else {
            unsafe { self.api.SubscribeMarketData(ptr, len) }
        };
        if rtn != 0 {
            return Err(format!("Fail to req `md_api_subscribe_market_data`: {}", rtn))
        }

        Ok(())
    }
}

impl Drop for CTPMDApi {
    fn drop(&mut self) {
        println!("drop api");
        unsafe { self.api.destruct(); }
        if let Some(spi) = self.spi {
            println!("drop spi");
            Self::drop_spi(spi);
        }
    }
}

fn main() {
    println!("load mdapi: {}", CTPMDApi::get_version());    
    let front_addr = "tcp://121.37.80.177:20004".to_string();
    // let front_addr = "tcp://180.168.146.187:10100".to_string();
    let mut api = CTPMDApi::new(
        "./".to_string(), 
        front_addr,
        "".to_string(),
        false,
        false);
    api.start().unwrap();
    api.subscribe_market_data(&["CU2310"], false).unwrap();
    let mut count = 0;

    println!("md api success");
    if let Some(ref mut rx) = api.rx {
        while let Ok(event) = rx.recv() {
            count += 1;
            println!("Got event: {:?}", event);
            if count >= 5 { break; }
        }
    }
}

