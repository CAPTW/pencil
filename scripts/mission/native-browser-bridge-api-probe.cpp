#include "../../src-tauri/native/browser_bridge.hpp"
class AbiServer {
    HMODULE module_=nullptr;uint64_t cookie_=0;
    using Create=uint32_t(WINAPI*)(uint64_t*);
    using Poll=uint32_t(WINAPI*)(uint64_t,uint32_t*,uint32_t*,uint64_t*,WCHAR*,uint32_t,uint32_t*);
    using Respond=uint32_t(WINAPI*)(uint64_t,uint32_t,const WCHAR*,uint32_t);
    using Close=uint32_t(WINAPI*)(uint64_t);
    Create create_=nullptr;Poll poll_=nullptr;Respond respond_=nullptr;Close close_=nullptr;
    template<class T>T symbol(const char* name){auto p=GetProcAddress(module_,name);if(!p)throw 30;return reinterpret_cast<T>(p);}
public:
    // Trust is compiled into the real DLL; test arguments cannot override it.
    AbiServer(const std::wstring&,const grammar::ApplyImageDigest&) {
        WCHAR path[32768]{};GetModuleFileNameW(nullptr,path,32768);
        std::wstring dll(path);dll=dll.substr(0,dll.find_last_of(L"\\/")+1)+L"target-apply.dll";
        module_=LoadLibraryExW(dll.c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
        if(!module_)throw 31;
        auto version=symbol<uint32_t(WINAPI*)()>("GrammarBrowserServerVersion");if(version()!=1)throw 32;
        create_=symbol<Create>("GrammarBrowserServerCreate");poll_=symbol<Poll>("GrammarBrowserServerPoll");
        respond_=symbol<Respond>("GrammarBrowserServerRespond");close_=symbol<Close>("GrammarBrowserServerClose");
        if(create_(&cookie_) || !cookie_)throw 33;
        uint64_t duplicate=9;if(create_(&duplicate)!=2 || duplicate)throw 34;
        WCHAR buffer[8193]{};uint32_t state=99,command=99,units=99;uint64_t capture=99;
        if(poll_(cookie_+1,&state,&command,&capture,buffer,8193,&units)!=1 || state!=2 || command || capture || units)throw 35;
        if(poll_(cookie_,&state,&command,&capture,buffer,8192,&units)!=3 || close_(cookie_+1)!=1)throw 36;
        if(respond_(cookie_,3,L"must not accompany an outcome",1)!=3)throw 37;
    }
    ~AbiServer(){if(cookie_)close_(cookie_);if(module_)FreeLibrary(module_);}
    void close(){if(cookie_){auto id=cookie_;if(close_(id))throw 38;cookie_=0;if(close_(id)!=1)throw 39;}}
    grammar::BrowserPoll poll(grammar::ChannelRequest& out) {
        out.clear();if(!cookie_)return grammar::BrowserPoll::closed;
        WCHAR buffer[8193]{};uint32_t state=2,command=0,units=0;uint64_t capture=0;
        auto code=poll_(cookie_,&state,&command,&capture,buffer,8193,&units);if(code)throw 40;
        out.header.command=static_cast<grammar::ChannelCommand>(command);out.header.capture_id=capture;
        out.body.resize(units*sizeof(wchar_t));if(units)memcpy(out.body.data(),buffer,out.body.size());
        SecureZeroMemory(buffer,sizeof(buffer));return state==0?grammar::BrowserPoll::idle:state==1?grammar::BrowserPoll::request:grammar::BrowserPoll::closed;
    }
    bool respond(uint32_t status,const std::wstring& text=L""){return respond_(cookie_,status,text.data(),static_cast<uint32_t>(text.size()))==0;}
};
#define BrowserBridgeServer AbiServer
#define GRAMMAR_ABI_PROBE
#include "native-browser-bridge-probe.cpp"
