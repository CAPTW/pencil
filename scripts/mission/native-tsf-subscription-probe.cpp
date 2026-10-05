// Inert COM source: exercises the production subscription and Word sink owners.
#include "../../src-tauri/native/word_tsf.hpp"
#include <thread>
#include <cstdio>
using namespace grammar;
struct Source final:ITfSource {
    LONG refs=1;unsigned advised=0,unadvised=0;DWORD returned=42;
    HRESULT adviseResult=S_OK,closeResult=S_OK;
    ComPtr<IUnknown> held;
    std::function<void()> onClose;
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_ITfSource){*out=static_cast<ITfSource*>(this);AddRef();return S_OK;}return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{return InterlockedDecrement(&refs);}
    HRESULT STDMETHODCALLTYPE AdviseSink(REFIID id,IUnknown* sink,DWORD* cookie) override {
        ++advised;if(id!=IID_ITfTextEditSink)return E_INVALIDARG;
        *cookie=returned;
        if(SUCCEEDED(adviseResult) || returned!=TF_INVALID_COOKIE)held=sink;
        return adviseResult;
    }
    HRESULT STDMETHODCALLTYPE UnadviseSink(DWORD cookie) override {
        ++unadvised;if(cookie!=42)return E_INVALIDARG;
        if(onClose)onClose();
        if(closeResult==S_OK || closeResult==CONNECT_E_NOCONNECTION)held.Reset();
        return closeResult;
    }
};
int wmain() {
    unsigned checks=0;
    auto require=[&](bool value){++checks;if(!value)printf("FAIL check=%u\n",checks);return value;};
    for(unsigned scenario=0;scenario<7;++scenario) {
        Source source;auto state=std::make_shared<WordTsfState>();
        auto lifetime=std::make_shared<int>(0);std::weak_ptr<void> retained=lifetime;
        ComPtr<WordTsfRevisionSink> sink;sink.Attach(new WordTsfRevisionSink(state,lifetime));lifetime.reset();
        {
            TsfSinkSubscription subscription;
            if(scenario==3){source.adviseResult=E_FAIL;source.returned=TF_INVALID_COOKIE;}
            if(scenario==4)source.adviseResult=S_FALSE;
            if(!require(subscription.attach(&source,IID_ITfTextEditSink,sink.Get())==(scenario!=3 && scenario!=4)))return 1;
            if(!require(!subscription.attach(&source,IID_ITfTextEditSink,sink.Get()) && source.advised==1))return 1;
            sink.Reset();
            if(scenario==3){if(!require(subscription.disconnected() && subscription.close() && source.unadvised==0 && retained.expired()))return 1;continue;}
            if(!require(!subscription.disconnected() && !retained.expired()))return 1;
            if(scenario==1 || scenario==2) {
                source.closeResult=scenario==1?E_FAIL:E_INVALIDARG;
                if(!require(!subscription.close() && !subscription.disconnected() && !retained.expired() && source.unadvised==1 && subscription.last_result()==source.closeResult))return 1;
                source.closeResult=S_OK;
            }
            if(scenario==5) {
                bool denied=false;std::thread other([&]{denied=!subscription.close();});other.join();
                if(!require(denied && source.unadvised==0 && !retained.expired()))return 1;
            }
            if(scenario==6) {
                bool reentrantDenied=false;
                source.onClose=[&]{reentrantDenied=!subscription.close() && !subscription.disconnected();};
                if(!require(subscription.close() && reentrantDenied && source.unadvised==1))return 1;
                source.onClose={};
            }
            if(!require(subscription.close() && subscription.disconnected() && retained.expired()))return 1;
            const auto count=source.unadvised;
            if(!require(subscription.close() && source.unadvised==count && source.refs==1))return 1;
        }
        if(!require(source.refs==1 && !source.held))return 1;
    }
    {
        Source source;source.closeResult=E_FAIL;
        auto state=std::make_shared<WordTsfState>();auto lifetime=std::make_shared<int>(0);std::weak_ptr<void> retained=lifetime;
        {
            ComPtr<WordTsfRevisionSink> sink;sink.Attach(new WordTsfRevisionSink(state,lifetime));lifetime.reset();
            TsfSinkSubscription subscription;
            if(!require(subscription.attach(&source,IID_ITfTextEditSink,sink.Get())))return 1;
        }
        state.reset();
        if(!require(source.unadvised==1 && source.refs==1 && !retained.expired()))return 1;
        ComPtr<ITfTextEditSink> callback;source.held.As(&callback);
        if(!require(callback && callback->OnEndEdit(nullptr,0,nullptr)==S_OK))return 1;
        callback.Reset();source.held.Reset();
        if(!require(retained.expired()))return 1;
    }
    for(bool missingCookie:{false,true}) {
        Source source;auto state=std::make_shared<WordTsfState>();auto lifetime=std::make_shared<int>(0);std::weak_ptr<void> retained=lifetime;
        ComPtr<WordTsfRevisionSink> sink;sink.Attach(new WordTsfRevisionSink(state,lifetime));lifetime.reset();
        if(missingCookie)source.returned=TF_INVALID_COOKIE;else source.closeResult=CONNECT_E_NOCONNECTION;
        {
            TsfSinkSubscription subscription;
            if(!require(subscription.attach(&source,IID_ITfTextEditSink,sink.Get())!=missingCookie))return 1;
            sink.Reset();
            if(!require(subscription.close()!=missingCookie && subscription.disconnected()!=missingCookie))return 1;
            if(missingCookie && !require(source.unadvised==0 && !retained.expired()))return 1;
        }
        source.held.Reset();if(!require(retained.expired() && source.refs==1))return 1;
    }
    printf("PASS %u production subscription/lifetime checks; no GUI or Office\n",checks);return 0;
}
