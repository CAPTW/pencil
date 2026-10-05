// Owned isolated RichEdit store, NOT Word/Office or installed-app qualification.
#include "../../src-tauri/native/tsf_selected_range.hpp"
#include "../../src-tauri/native/word_tsf.hpp"
#include "word-metadata-fixture.hpp"
#include <richedit.h>
#include <cstdio>
using namespace grammar;
static void pump(DWORD ms) {
    const auto end=GetTickCount64()+ms;
    do {MSG msg;while(PeekMessageW(&msg,nullptr,0,0,PM_REMOVE)){TranslateMessage(&msg);DispatchMessageW(&msg);}Sleep(5);}while(GetTickCount64()<end);
}
struct Session final:ITfEditSession {
    LONG refs=1;bool done=false;HRESULT result=E_PENDING;std::function<HRESULT(TfEditCookie)> work;
    explicit Session(std::function<HRESULT(TfEditCookie)> f):work(std::move(f)){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_ITfEditSession){*out=static_cast<ITfEditSession*>(this);AddRef();return S_OK;}return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs);if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
        try{result=work(cookie);}catch(...){result=E_FAIL;}done=true;return result;
    }
};
static bool run(ITfContext* context,TfClientId client,DWORD flags,std::function<HRESULT(TfEditCookie)> work,HRESULT* disposition=nullptr) {
    auto session=new Session(std::move(work));HRESULT result=E_PENDING;
    const auto requested=context->RequestEditSession(client,session,TF_ES_ASYNC|flags,&result);
    if(disposition)*disposition=result;
    printf("request=%08lx disposition=%08lx flags=%lu done=%d\n",requested,result,flags,session->done);fflush(stdout);
    const bool queued=SUCCEEDED(requested) && result==TF_S_ASYNC;
    for(unsigned n=0;n<500 && !session->done && queued;++n)pump(10);
    const bool ok=SUCCEEDED(requested) && session->done && SUCCEEDED(session->result);
    // Do not continue and destroy callback captures if a store retained a late
    // callback. This isolated process terminates; it owns no external documents.
    if(!session->done && queued){puts("FAIL pending callback deadline");fflush(stdout);ExitProcess(9);}
    session->Release();return ok;
}
static std::wstring readFixture(HWND edit) {
    const int count=GetWindowTextLengthW(edit);std::vector<WCHAR> out(static_cast<size_t>(count)+1);
    GetWindowTextW(edit,out.data(),count+1);return std::wstring(out.data());
}
struct Port final:WordNativeWindowPort {
    Fixture* fixture;HWND field,root;bool live=true;
    Port(Fixture* f,HWND edit,HWND top):fixture(f),field(edit),root(top){}
    bool eligible(HWND edit,HWND top,DWORD thread) override{return live && edit==field && top==root && thread==GetCurrentThreadId();}
    HRESULT native_window(HWND edit,IDispatch** out) override {
        *out=nullptr;if(edit!=field)return E_INVALIDARG;*out=&fixture->window;fixture->window.AddRef();return S_OK;
    }
};
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount"){puts("REFUSED Sandbox only");return 2;}
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int exitCode=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client)))return 4;
        const auto library=LoadLibraryW(L"Msftedit.dll");
        HWND top=CreateWindowExW(0,L"STATIC",L"Grammar owned correlation probe",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,650,300,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        HWND edit=CreateWindowExW(0,L"RICHEDIT50W",L"hello world suffix world",WS_CHILD|WS_VISIBLE|ES_MULTILINE|ES_AUTOVSCROLL,10,10,600,200,top,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!library || !top || !edit)return 5;
        SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);
        SetForegroundWindow(top);SetFocus(edit);pump(300);
        ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
        if(FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context)return 6;
        for(unsigned scenario=0;scenario<14;++scenario) {
            const std::wstring original=L"hello world suffix world";
            SetWindowTextW(edit,original.c_str());SendMessageW(edit,EM_SETSEL,6,11);pump(20);
            Fixture f;f.hwnd=top;f.window.number(L"Hwnd",static_cast<LONG>(reinterpret_cast<INT_PTR>(top)));
            f.window.object(L"Application",&f.app);f.range.number(L"Start",6);f.range.number(L"End",11);
            f.range.allowText=true;f.range.text=L"world";f.range.values[L"Text"]={VT_BSTR,0,nullptr};
            auto port=std::make_shared<Port>(&f,edit,top);
            unsigned reads=0,mutations=0;bool ok=true;
            {
                WordTsfRevisionBinding binding(context.Get(),scenario==3?top:edit,top,port,true);
                if(scenario==1)f.range.text=L"other";
                if(scenario==2)f.document.flag(L"HasPassword",true);
                if(scenario==12)f.range.wrong=L"Text";
                if(scenario==13)f.range.onInvoke=[&](const std::wstring& name){if(name==L"Text")f.document.flag(L"ReadOnly",true);};
                std::shared_ptr<WordCorrelationLease> lease;
                ok=run(context.Get(),client,TF_ES_READ,[&](TfEditCookie cookie){lease=WordCorrelationLease::capture(binding.state,scenario==11?TF_INVALID_COOKIE:cookie,reads);return S_OK;});
                const bool refusedCapture=scenario==1 || scenario==2 || scenario==3 || scenario>=11;
                ok=ok && static_cast<bool>(lease)!=refusedCapture;
                if(refusedCapture) {
                    ok=ok && readFixture(edit)==original;
                    if(scenario==2 || scenario==3 || scenario==11)ok=ok && f.range.textReads==0 && reads==0;
                } else if(lease) {
                    ok=ok && lease->source()==L"world";
                    const auto revision=binding.state->revision.load();
                    if(scenario==4){SetWindowTextW(edit,L"hello changed");SetWindowTextW(edit,original.c_str());SendMessageW(edit,EM_SETSEL,6,11);pump(20);ok=ok && binding.state->revision.load()>revision;}
                    if(scenario==5)f.range.number(L"Start",7);
                    if(scenario==6)f.range.text=L"other";
                    if(scenario==7)binding.disable();
                    if(scenario==8)lease->cancel();
                    std::shared_ptr<WordCorrelationLease> newer;
                    if(scenario==9)ok=ok && run(context.Get(),client,TF_ES_READ,[&](TfEditCookie cookie){newer=WordCorrelationLease::capture(binding.state,cookie,reads);return S_OK;}) && newer;
                    if(scenario==10)SendMessageW(edit,EM_SETSEL,19,24);
                    ApplyResult result=ApplyResult::pending;
                    ok=ok && run(context.Get(),client,TF_ES_READWRITE,[&](TfEditCookie cookie){result=lease->accept(cookie,L"reviewed",reads,mutations);return S_OK;});
                    ok=ok && result==(scenario==0?ApplyResult::applied:ApplyResult::rejected) && mutations==(scenario==0?1u:0u) &&
                        readFixture(edit)==(scenario==0?L"hello reviewed suffix world":original);
                    const auto before=mutations;
                    ok=ok && run(context.Get(),client,TF_ES_READWRITE,[&](TfEditCookie cookie){return lease->accept(cookie,L"replay",reads,mutations)==ApplyResult::rejected?S_OK:E_FAIL;}) && mutations==before;
                }
                printf("case=%u pass=%d tsf_reads=%u word_selected_reads=%u mutations=%u revision=%llu\n",scenario,ok,reads,f.range.textReads,mutations,binding.state->revision.load());
            }
            if(!f.balanced())ok=false;
            if(!ok)exitCode=1;
        }
        context.Reset();document.Reset();DestroyWindow(top);manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("result=%s scope=real_TSF_plus_inert_Word_OM Office=NOT_RUN\n",exitCode?"FAIL":"PASS");return exitCode;
}
