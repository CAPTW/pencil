#include "word-metadata-fixture.hpp"
#include "../../src-tauri/native/word_target.hpp"
struct NativePort final:WordNativeWindowPort {
    Node* window=nullptr;
    bool allowed=true;HRESULT result=S_OK;
    unsigned checks=0,calls=0;
    std::function<void()> onNative;
    bool eligible(HWND field,HWND root,DWORD thread) override {
        ++checks;
        return allowed && field==reinterpret_cast<HWND>(4321) && root==reinterpret_cast<HWND>(1234) && thread==GetCurrentThreadId();
    }
    HRESULT native_window(HWND field,IDispatch** out) override {
        ++calls;*out=nullptr;
        if(field!=reinterpret_cast<HWND>(4321))return E_INVALIDARG;
        if(onNative)onNative();
        if(window){*out=window;window->AddRef();}return result;
    }
};
int wmain() {
    unsigned checks=0;
    auto require=[&](bool ok){++checks;if(!ok)printf("FAIL check=%u\n",checks);return ok;};
    for(unsigned scenario=0;scenario<18;++scenario) {
        Fixture f;f.window.object(L"Application",&f.app);
        auto port=std::make_shared<NativePort>();port->window=&f.window;
        const WordObservationGuard guard{f.thread,scenario==10?0:GetTickCount64()+5000,7,f.revision,f.enabled};
        switch(scenario) {
        case 1:port->allowed=false;break;
        case 2:port->window=nullptr;break;
        case 3:port->result=E_NOINTERFACE;break;
        case 4:f.window.number(L"Hwnd",4321);break;
        case 5:f.window.object(L"Application",nullptr);break;
        case 6:f.app.object(L"ActiveWindow",&f.other);break;
        case 7:f.document.flag(L"HasPassword",true);break;
        case 8:f.enabled=false;break;
        case 9:++f.revision;break;
        case 10:break;
        case 11:port->onNative=[&]{f.enabled=false;};break;
        case 12:port->onNative=[&]{++f.revision;};break;
        case 13:port->onNative=[&]{port->allowed=false;};break;
        case 14:f.window.wrong=L"Application";break;
        case 15:f.window.missing=L"Application";break;
        case 16:f.window.failed=L"Hwnd";break;
        case 17:port->onNative=[&]{if(port->calls==2)f.range.number(L"Start",5);};break;
        }
        auto target=WordTargetIdentity::capture(port,reinterpret_cast<HWND>(4321),f.hwnd,guard);
        if(!require((target!=nullptr)==(scenario==0)))return 1;
        if(scenario==0) {
            if(!require(target->matches(guard)))return 1;
            f.range.onInvoke=[&](const std::wstring& name){if(name==L"End")port->allowed=false;};
            if(!require(!target->matches(guard)))return 1;
            f.range.onInvoke={};port->allowed=true;
            port->window=&f.other;
            if(!require(!target->matches(guard)))return 1;
        }
        if(scenario==1 || scenario==8 || scenario==9 || scenario==10)if(!require(port->calls==0))return 1;
        if(scenario==7)if(!require(f.selection.invokes==0 && f.range.invokes==0))return 1;
        target.reset();if(!require(f.balanced()))return 1;
    }
    {
        Fixture f;f.window.object(L"Application",&f.app);
        auto port=std::make_shared<NativePort>();port->window=&f.window;auto guard=f.guard();
        bool refused=false;
        std::thread other([&]{refused=!WordTargetIdentity::capture(port,reinterpret_cast<HWND>(4321),f.hwnd,guard);});other.join();
        if(!require(refused && port->checks==0 && port->calls==0 && f.balanced()))return 1;
        auto target=WordTargetIdentity::capture(port,reinterpret_cast<HWND>(4321),f.hwnd,guard);
        f.app.object(L"ActiveWindow",&f.other);
        if(!require(target && !target->matches(guard)))return 1;
        target.reset();if(!require(f.balanced()))return 1;
    }
    WordNativeWindowWin32 actual;
    if(!require(!actual.eligible(nullptr,nullptr,GetCurrentThreadId())))return 1;
    printf("PASS %u Word target acquisition checks; Office and native OM live NOT_RUN; text reads 0\n",checks);
    return 0;
}
