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
int wmain(){
    unsigned checks=0;auto require=[&](bool value){if(!value){printf("FAIL %u\n",checks+1);return false;}++checks;return true;};
    {
        Fixture f;auto identity=f.capture();if(!require(identity && identity->matches(f.guard())))return 1;
        f.selection.object(L"Document",&f.alias);if(!require(identity->matches(f.guard())))return 1;
        f.range.number(L"Start",5);if(!require(!identity->matches(f.guard())))return 1;
        identity.reset();if(!require(f.balanced()))return 1;
    }
    for(unsigned scenario=0;scenario<26;++scenario){
        Fixture f;
        switch(scenario){
        case 0:f.enabled=false;break;
        case 1:f.revision=8;break;
        case 2:f.hwnd=nullptr;break;
        case 3:f.window.number(L"Hwnd",4321);break;
        case 4:f.app.object(L"ActiveDocument",&f.other);break;
        case 5:f.selection.object(L"Document",&f.other);break;
        case 6:f.range.object(L"Document",&f.other);break;
        case 7:f.selection.number(L"Type",1);break;
        case 8:f.selection.number(L"Type",6);break;
        case 9:f.range.number(L"StoryType",2);break;
        case 10:f.range.number(L"Start",-1);break;
        case 11:f.range.number(L"End",4);break;
        case 12:f.range.number(L"End",3);break;
        case 13:f.range.number(L"End",INT32_MAX);break;
        case 14:f.font.number(L"Hidden",-1);break;
        case 15:f.font.number(L"Hidden",9999999);break;
        case 16:f.fields.number(L"Count",1);break;
        case 17:f.controls.number(L"Count",1);break;
        case 18:f.tables.number(L"Count",1);break;
        case 19:f.shapes.number(L"Count",1);break;
        case 20:f.document.flag(L"HasPassword",true);break;
        case 21:f.document.flag(L"WriteReserved",true);break;
        case 22:f.document.flag(L"ReadOnly",true);break;
        case 23:f.document.number(L"ProtectionType",3);break;
        case 24:f.permission.flag(L"Enabled",true);break;
        case 25:f.window.object(L"Selection",nullptr);break;
        }
        if(!require(!f.capture() && f.balanced()))return 1;
        if(scenario>=20 && scenario<=24 && !require(f.selection.invokes==0 && f.range.invokes==0))return 1;
    }
    for(unsigned fault=0;fault<3;++fault){
        Fixture f;
        for(auto pair: {std::make_pair(&f.app,L"ActiveWindow"),{&f.window,L"Hwnd"},{&f.range,L"Start"},{&f.font,L"Hidden"},{&f.fields,L"Count"}}){
            auto* property=fault==0?&pair.first->missing:fault==1?&pair.first->wrong:&pair.first->failed;
            *property=pair.second;if(!require(!f.capture() && f.balanced()))return 1;property->clear();
        }
    }
    {
        Fixture f;const auto expired=WordObservationGuard{f.thread,GetTickCount64(),7,f.revision,f.enabled};
        if(!require(!WordSelectionIdentity::capture(&f.app,f.hwnd,expired) && !f.app.invokes))return 1;
        bool refused=false;std::thread worker([&]{refused=!f.capture();});worker.join();
        if(!require(refused && !f.app.invokes && f.balanced()))return 1;
    }
    for(unsigned scenario=0;scenario<4;++scenario){
        Fixture f;unsigned observed=0;
        f.range.onInvoke=[&](const std::wstring& name){if(name!=L"End")return;
            if(scenario==0){++f.revision;} // Revision changes even if range/source restored.
            if(scenario==1){f.enabled=false;}
            if(scenario==2 && ++observed==2){f.range.number(L"End",17);}
            if(scenario==3){f.document.flag(L"ReadOnly",true);}
        };
        if(!require(!f.capture() && f.balanced()))return 1;
    }
    {
        Fixture f;auto identity=f.capture();if(!require(static_cast<bool>(identity)))return 1;
        ++f.revision;auto newer=WordObservationGuard{f.thread,GetTickCount64()+5000,8,f.revision,f.enabled};
        if(!require(!identity->matches(newer)))return 1;
        identity.reset();if(!require(f.balanced()))return 1;
    }
    printf("PASS %u inert Word selection checks; text reads 0; Office NOT_RUN\n",checks);return 0;
}
