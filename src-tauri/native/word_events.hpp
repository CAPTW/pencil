#pragma once
#include "word_tsf.hpp"
#include <ocidl.h>

namespace grammar {
// Microsoft Word ApplicationEvents4 dispinterface. No event arguments are read.
inline constexpr GUID word_application_events = {0x00020a01,0x0000,0x0000,{0xc0,0x00,0x00,0x00,0x00,0x00,0x00,0x46}};
class WordApplicationEventSink final : public IDispatch {
    LONG refs_ = 1;
    std::weak_ptr<WordTsfState> state_;
    std::shared_ptr<void> lifetime_;
    const DWORD thread_;
    const std::shared_ptr<WordInvalidationSignal> signal_;
public:
    WordApplicationEventSink(const std::shared_ptr<WordTsfState>& state,std::shared_ptr<void> lifetime)
        :state_(state),lifetime_(std::move(lifetime)),thread_(state->thread),signal_(state->signal){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_IDispatch || id==word_application_events){*out=static_cast<IDispatch*>(this);AddRef();return S_OK;}
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs_);}
    ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs_);if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE GetTypeInfoCount(UINT* count) override{if(!count)return E_POINTER;*count=0;return S_OK;}
    HRESULT STDMETHODCALLTYPE GetTypeInfo(UINT,LCID,ITypeInfo**) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE GetIDsOfNames(REFIID,LPOLESTR*,UINT,LCID,DISPID*) override{return DISP_E_UNKNOWNNAME;}
    HRESULT STDMETHODCALLTYPE Invoke(DISPID,REFIID,LCID,WORD,DISPPARAMS*,VARIANT*,EXCEPINFO*,UINT*) override {
        if(GetCurrentThreadId()!=thread_) { signal_->stop();return RPC_E_WRONG_THREAD; }
        if(auto state=state_.lock())state->invalidate();
        // Do not dereference selection/document/ByRef Cancel or diagnostic args.
        // Every event conservatively invalidates, including unknown newer ones.
        return S_OK;
    }
};
// Adapts the existing checked cookie owner to a COM connection point.
class WordEventSource final : public ITfSource {
    LONG refs_=1;
    ComPtr<IConnectionPoint> point_;
    DWORD actual_cookie_=0;
public:
    explicit WordEventSource(IConnectionPoint* point):point_(point){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_ITfSource){*out=static_cast<ITfSource*>(this);AddRef();return S_OK;}return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs_);}
    ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs_);if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE AdviseSink(REFIID id,IUnknown* sink,DWORD* cookie) override {
        if(!cookie)return E_POINTER;*cookie=TF_INVALID_COOKIE;
        if(id!=word_application_events || !point_)return E_NOINTERFACE;
        if(actual_cookie_)return CONNECT_E_ADVISELIMIT;
        IID interfaceId{};
        if(point_->GetConnectionInterface(&interfaceId)!=S_OK || interfaceId!=word_application_events)return E_NOINTERFACE;
        DWORD actual=0;const auto hr=point_->Advise(sink,&actual);
        // IConnectionPoint uses zero for failed registration; ITfSource uses
        // INVALID_COOKIE. Keep every nonzero native cookie, including UINT_MAX.
        if(actual){actual_cookie_=actual;*cookie=1;}
        return hr;
    }
    HRESULT STDMETHODCALLTYPE UnadviseSink(DWORD cookie) override {
        if(!point_ || cookie!=1 || !actual_cookie_)return E_INVALIDARG;
        const auto hr=point_->Unadvise(actual_cookie_);
        if(hr==S_OK || hr==CONNECT_E_NOCONNECTION)actual_cookie_=0;
        return hr;
    }
};
class WordEventBinding final {
    TsfSinkSubscription subscription_;
    ComPtr<IDispatch> application_;
    bool attached_=false;
public:
    bool attach(const std::shared_ptr<WordTsfState>& state,const WordTargetIdentity& target,
        const WordObservationGuard& guard,std::shared_ptr<void> lifetime) {
        if(!state || !lifetime || !guard.current() || !target.matches(guard))return false;
        ComPtr<IConnectionPointContainer> container;ComPtr<IConnectionPoint> point;
        if(target.application_->QueryInterface(IID_PPV_ARGS(&container))!=S_OK || !container ||
            container->FindConnectionPoint(word_application_events,&point)!=S_OK || !point || !guard.current())return false;
        ComPtr<WordEventSource> source;source.Attach(new WordEventSource(point.Get()));
        ComPtr<WordApplicationEventSink> sink;sink.Attach(new WordApplicationEventSink(state,std::move(lifetime)));
        const bool subscribed=subscription_.attach(source.Get(),word_application_events,sink.Get());
        attached_=subscribed && guard.current() && target.matches(guard);
        if(attached_)application_=target.application_;
        return attached_;
    }
    bool matches(IDispatch* application) const {
        return attached_ && !subscription_.disconnected() && word_selection_detail::same(application_.Get(),application);
    }
    bool close(){attached_=false;if(!subscription_.close())return false;application_.Reset();return true;}
    ~WordEventBinding(){close();}
};
}
