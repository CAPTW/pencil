#pragma once
#include "word-metadata-fixture.hpp"
#include "../../src-tauri/native/word_events.hpp"
struct Events final:IConnectionPoint,IConnectionPointContainer {
    LONG refs=1;unsigned advises=0,unadvises=0;
    DWORD nativeCookie=42;
    HRESULT adviseResult=S_OK,closeResult=S_OK;
    bool missing=false,wrongInterface=false;
    ComPtr<IDispatch> sink;
    std::function<void()> onAdvise;
    std::function<void()> onFind;
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_IConnectionPoint)*out=static_cast<IConnectionPoint*>(this);
        else if(id==IID_IConnectionPointContainer)*out=static_cast<IConnectionPointContainer*>(this);
        else return E_NOINTERFACE;
        AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{return InterlockedDecrement(&refs);}
    HRESULT STDMETHODCALLTYPE GetConnectionInterface(IID* id) override {if(!id)return E_POINTER;*id=wrongInterface?IID_IDispatch:word_application_events;return S_OK;}
    HRESULT STDMETHODCALLTYPE GetConnectionPointContainer(IConnectionPointContainer** out) override {if(!out)return E_POINTER;*out=this;AddRef();return S_OK;}
    HRESULT STDMETHODCALLTYPE Advise(IUnknown* value,DWORD* cookie) override {
        ++advises;*cookie=0;
        if(FAILED(adviseResult))return adviseResult;
        if(value->QueryInterface(word_application_events,reinterpret_cast<void**>(sink.GetAddressOf()))!=S_OK)return CONNECT_E_CANNOTCONNECT;
        *cookie=nativeCookie;if(onAdvise)onAdvise();return adviseResult;
    }
    HRESULT STDMETHODCALLTYPE Unadvise(DWORD cookie) override {
        ++unadvises;if(cookie!=nativeCookie)return E_INVALIDARG;
        if(closeResult==S_OK || closeResult==CONNECT_E_NOCONNECTION)sink.Reset();return closeResult;
    }
    HRESULT STDMETHODCALLTYPE EnumConnections(IEnumConnections**) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE EnumConnectionPoints(IEnumConnectionPoints**) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE FindConnectionPoint(REFIID id,IConnectionPoint** out) override {
        if(onFind)onFind();
        *out=nullptr;if(missing || id!=word_application_events)return CONNECT_E_NOCONNECTION;
        *out=this;AddRef();return S_OK;
    }
    HRESULT fire(DISPID id=12) {return sink?sink->Invoke(id,IID_NULL,0,DISPATCH_METHOD,nullptr,nullptr,nullptr,nullptr):S_FALSE;}
};
