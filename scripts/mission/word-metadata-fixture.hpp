// Inert Word-shaped COM graph. Does not start or attach to an Office process.
#include "../../src-tauri/native/word_selection.hpp"
#include <map>
#include <string>
#include <functional>
#include <thread>
#include <cstdio>
using namespace grammar;
struct Node final:IDispatch {
    struct Property {VARTYPE type;LONG number;Node* object;};
    LONG refs=1;
    unsigned invokes=0,forbidden=0;
    Node* identity=this;
    std::map<std::wstring,Property> values;
    std::map<DISPID,std::wstring> names;
    std::wstring missing,wrong,failed;
    std::function<void(const std::wstring&)> onInvoke;
    void number(const wchar_t* name,LONG value){values[name]={VT_I4,value,nullptr};}
    void flag(const wchar_t* name,bool value){values[name]={VT_BOOL,value?VARIANT_TRUE:VARIANT_FALSE,nullptr};}
    void object(const wchar_t* name,Node* value){values[name]={VT_DISPATCH,0,value};}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown){*out=static_cast<IUnknown*>(identity);identity->AddRef();return S_OK;}
        if(id==IID_IDispatch){*out=static_cast<IDispatch*>(this);AddRef();return S_OK;}
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{return InterlockedDecrement(&refs);}
    HRESULT STDMETHODCALLTYPE GetTypeInfoCount(UINT*) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE GetTypeInfo(UINT,LCID,ITypeInfo**) override{return E_NOTIMPL;}
    HRESULT STDMETHODCALLTYPE GetIDsOfNames(REFIID,LPOLESTR* requested,UINT count,LCID,DISPID* result) override {
        if(count!=1)return E_INVALIDARG;
        const std::wstring name=requested[0];
        if(name==L"Text" || name==L"Content" || name==L"Password" || name==L"WritePassword" || name==L"FullName")++forbidden;
        if(name==missing || !values.count(name))return DISP_E_UNKNOWNNAME;
        *result=static_cast<DISPID>(names.size()+1);names[*result]=name;return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Invoke(DISPID id,REFIID,LCID,WORD flags,DISPPARAMS* args,VARIANT* out,EXCEPINFO*,UINT*) override {
        ++invokes;
        if(flags!=DISPATCH_PROPERTYGET || args->cArgs || args->cNamedArgs){++forbidden;return E_INVALIDARG;}
        const auto name=names.at(id);
        if(onInvoke)onInvoke(name);
        if(name==failed)return DISP_E_EXCEPTION;
        const auto value=values.at(name);
        if(name==wrong){out->vt=VT_EMPTY;return S_OK;}
        out->vt=value.type;
        if(value.type==VT_DISPATCH){out->pdispVal=value.object;if(value.object)value.object->AddRef();}
        else if(value.type==VT_BOOL)out->boolVal=static_cast<VARIANT_BOOL>(value.number);
        else out->lVal=value.number;
        return S_OK;
    }
};
struct Fixture {
    Node app,window,document,selection,range,permission,font,fields,controls,tables,shapes,other,alias;
    std::atomic<uint64_t> revision{7};std::atomic<bool> enabled{true};
    const DWORD thread=GetCurrentThreadId();
    HWND hwnd=reinterpret_cast<HWND>(static_cast<INT_PTR>(1234));
    Fixture(){
        app.object(L"ActiveWindow",&window);app.object(L"ActiveDocument",&document);
        window.number(L"Hwnd",1234);window.object(L"Document",&document);window.object(L"Selection",&selection);
        document.flag(L"HasPassword",false);document.flag(L"WriteReserved",false);document.flag(L"ReadOnly",false);
        document.number(L"ProtectionType",-1);document.object(L"Permission",&permission);permission.flag(L"Enabled",false);
        selection.object(L"Document",&document);selection.number(L"Type",2);selection.object(L"Range",&range);
        range.object(L"Document",&document);range.number(L"StoryType",1);range.number(L"Start",4);range.number(L"End",16);
        range.object(L"Font",&font);font.number(L"Hidden",0);
        range.object(L"Fields",&fields);fields.number(L"Count",0);
        range.object(L"ContentControls",&controls);controls.number(L"Count",0);
        range.object(L"Tables",&tables);tables.number(L"Count",0);
        range.object(L"InlineShapes",&shapes);shapes.number(L"Count",0);
        alias.identity=&document;
    }
    WordObservationGuard guard(){return {thread,GetTickCount64()+5000,7,revision,enabled};}
    std::unique_ptr<WordSelectionIdentity> capture(){return WordSelectionIdentity::capture(&app,hwnd,guard());}
    bool balanced(){for(auto* n:{&app,&window,&document,&selection,&range,&permission,&font,&fields,&controls,&tables,&shapes,&other,&alias})if(n->refs!=1 || n->forbidden)return false;return true;}
};
