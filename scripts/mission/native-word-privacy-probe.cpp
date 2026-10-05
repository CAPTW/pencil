// Inert IDispatch objects only. Never loads Office, user files or a UI.
#include <initializer_list>
#include "../../src-tauri/native/word_privacy.hpp"
#include <cstdio>
#include <string>
#include <map>
#include <thread>
using namespace grammar;
struct Fake final:IDispatch {
    LONG refs=1;unsigned calls=0,forbidden=0;
    bool password=false,write=false,read=false,enabled=false;
    LONG protection=-1;std::wstring missing,wrong,failed,noncanonical;
    Fake* permission=nullptr;
    std::map<DISPID,std::wstring> names;
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id!=IID_IUnknown && id!=IID_IDispatch)return E_NOINTERFACE;
        *out=static_cast<IDispatch*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{return InterlockedDecrement(&refs);}
    HRESULT STDMETHODCALLTYPE GetTypeInfoCount(UINT*) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE GetTypeInfo(UINT,LCID,ITypeInfo**) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE GetIDsOfNames(REFIID,LPOLESTR* values,UINT n,LCID,DISPID* out) override {
        if(n!=1)return E_INVALIDARG;
        const std::wstring name=values[0];
        const bool valid=name==L"HasPassword"||name==L"WriteReserved"||name==L"ReadOnly"||name==L"ProtectionType"||name==L"Permission"||name==L"Enabled";
        if(!valid){++forbidden;return DISP_E_UNKNOWNNAME;}
        if(name==missing)return DISP_E_UNKNOWNNAME;
        *out=static_cast<DISPID>(names.size()+1);names[*out]=name;return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Invoke(DISPID id,REFIID,LCID,WORD flags,DISPPARAMS* params,VARIANT* out,EXCEPINFO* error,UINT*) override {
        ++calls;
        if(flags!=DISPATCH_PROPERTYGET || !params || params->cArgs || params->cNamedArgs){++forbidden;return E_INVALIDARG;}
        const auto name=names.at(id);
        if(name==failed){error->bstrDescription=SysAllocString(L"synthetic error, not emitted");return DISP_E_EXCEPTION;}
        if(name==wrong){out->vt=VT_I2;out->iVal=0;return S_OK;}
        if(name==noncanonical){out->vt=VT_BOOL;out->boolVal=1;return S_OK;}
        if(name==L"Permission"){out->vt=VT_DISPATCH;out->pdispVal=permission;if(permission)permission->AddRef();return S_OK;}
        if(name==L"ProtectionType"){out->vt=VT_I4;out->lVal=protection;return S_OK;}
        out->vt=VT_BOOL;out->boolVal=(name==L"HasPassword"?password:name==L"WriteReserved"?write:name==L"ReadOnly"?read:enabled)?VARIANT_TRUE:VARIANT_FALSE;return S_OK;
    }
};
int wmain(){
    unsigned checks=0;
    auto require=[&](bool ok){if(!ok){printf("FAIL %u\n",checks+1);return false;}++checks;return true;};
    const auto thread=GetCurrentThreadId();
    Fake permission,doc;doc.permission=&permission;
    auto run=[&]{return word_privacy_metadata(&doc,thread,true);};
    if(!require(word_privacy_metadata(nullptr,thread,true)==WordPrivacyMetadata::unknown) ||
       !require(word_privacy_metadata(&doc,thread,false)==WordPrivacyMetadata::unknown && doc.calls==0) ||
       !require(word_privacy_metadata(&doc,0,true)==WordPrivacyMetadata::unknown && doc.calls==0) ||
       !require(run()==WordPrivacyMetadata::clear && permission.refs==1))return 1;
    for(auto flag:{&doc.password,&doc.write,&doc.read,&permission.enabled}){
        *flag=true;if(!require(run()==WordPrivacyMetadata::denied))return 1;*flag=false;
    }
    for(const LONG value:{0,1,2,3,42}){doc.protection=value;if(!require(run()==WordPrivacyMetadata::denied))return 1;}doc.protection=-1;
    for(const auto* name:{L"HasPassword",L"WriteReserved",L"ReadOnly",L"ProtectionType",L"Permission"}){
        doc.missing=name;if(!require(run()==WordPrivacyMetadata::unknown))return 1;doc.missing.clear();
        doc.wrong=name;if(!require(run()==WordPrivacyMetadata::unknown))return 1;doc.wrong.clear();
        doc.failed=name;if(!require(run()==WordPrivacyMetadata::unknown))return 1;doc.failed.clear();
    }
    permission.missing=L"Enabled";if(!require(run()==WordPrivacyMetadata::unknown))return 1;permission.missing.clear();
    permission.wrong=L"Enabled";if(!require(run()==WordPrivacyMetadata::unknown))return 1;permission.wrong.clear();
    permission.failed=L"Enabled";if(!require(run()==WordPrivacyMetadata::unknown))return 1;permission.failed.clear();
    doc.noncanonical=L"HasPassword";if(!require(run()==WordPrivacyMetadata::unknown))return 1;doc.noncanonical.clear();
    const auto beforeCalls=doc.calls;
    WordPrivacyMetadata otherThread=WordPrivacyMetadata::clear;
    std::thread other([&]{otherThread=word_privacy_metadata(&doc,thread,true);});other.join();
    if(!require(otherThread==WordPrivacyMetadata::unknown && doc.calls==beforeCalls))return 1;
    doc.permission=nullptr;if(!require(run()==WordPrivacyMetadata::unknown))return 1;doc.permission=&permission;
    if(!require(run()==WordPrivacyMetadata::clear && permission.refs==1 && doc.refs==1 && !doc.forbidden && !permission.forbidden))return 1;
    printf("PASS %u inert COM metadata checks; Office NOT_RUN\n",checks);return 0;
}
