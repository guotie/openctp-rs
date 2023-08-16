#include <iostream>
#include <chrono>
#include <thread>
#include "../md/ThostFtdcMdApi.h"

auto reqtime = std::chrono::steady_clock::now();

class CMarketSpi :public CThostFtdcMdSpi {
public:
	CMarketSpi(CThostFtdcMdApi* pApi):m_pMarketApi(pApi) {
		pApi->RegisterSpi(this);
	}
	void OnFrontConnected() {
		std::cout << "connected." << std::endl;
		reqtime = std::chrono::steady_clock::now();
		CThostFtdcReqUserLoginField Req = {0};
		m_pMarketApi->ReqUserLogin(&Req, 0);
	}
	void OnFrontDisconnected(int nReason) {
		std::cout << "disconnected." << std::endl;
		exit(0);
	}
	void OnRspUserLogin(CThostFtdcRspUserLoginField* pRspUserLogin, CThostFtdcRspInfoField* pRspInfo, int nRequestID, bool bIsLast) {
		auto rsptime = std::chrono::steady_clock::now();
		auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(rsptime - reqtime);
		std::cout << "login. response time: " << duration.count() << " milliseconds" << std::endl;
	}
	void OnRtnDepthMarketData(CThostFtdcDepthMarketDataField* pDepthMarketData) {
		std::cout << pDepthMarketData->InstrumentID << " - " << pDepthMarketData->LastPrice << " - " << pDepthMarketData->Volume << std::endl;
	}
	CThostFtdcMdApi* m_pMarketApi;
};

int main(int argc,char *argv[]) {
	if (argc != 2) {
		std::cout << "usage: ctpping {address}" << std::endl;
		std::cout << "example: ctpping tcp://121.37.80.177:20004" << std::endl;
		return 0;
	}
	std::cout << "version:" << CThostFtdcMdApi::GetApiVersion() << std::endl;

	CThostFtdcMdApi* pApi = CThostFtdcMdApi::CreateFtdcMdApi();
	CMarketSpi Spi(pApi);
	pApi->RegisterFront(argv[1]);
	pApi->RegisterSpi(&Spi);
	pApi->Init();

	const char* symbols[] = { "600000","600002","AG2312" };
	int ret = pApi->SubscribeMarketData((char **)symbols, sizeof(symbols)/sizeof(symbols[0]));
	std::cout << "subscribe return: " << ret << std::endl;

	std::this_thread::sleep_for(std::chrono::seconds(3));
	std::cout << "press ANY key to exit ..." << std::endl;
	getchar();

	return 0;
}
