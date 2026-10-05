// Owned restricted Sandbox only. Real TSF metadata, no fabricated plain scope.
#include "../../src-tauri/native/standard_edit.hpp"
#include "../../src-tauri/native/standard_edit_scope.hpp"
#include <cstdio>
using namespace grammar;
static void pump() { MSG m; while(PeekMessageW(&m,nullptr,0,0,PM_REMOVE)){TranslateMessage(&m);DispatchMessageW(&m);} }
class Session final : public ITfEditSession {
    LONG refs_=1;
public:
    bool completed=false, captured=false, scope=false;
    ApplyResult result=ApplyResult::rejected;
    std::function<void(TfEditCookie)> operation;
    explicit Session(std::function<void(TfEditCookie)> op):operation(std::move(op)) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER; *out=nullptr;
        if(id!=IID_IUnknown&&id!=IID_ITfEditSession)return E_NOINTERFACE;
        *out=static_cast<ITfEditSession*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override {return InterlockedIncrement(&refs_);}
    ULONG STDMETHODCALLTYPE Release() override {auto n=InterlockedDecrement(&refs_);if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
        try {operation(cookie);completed=true;return S_OK;}catch(...){completed=true;return E_FAIL;}
    }
};
static bool run(ITfContext* context,TfClientId client,Session* session,bool write) {
    HRESULT outcome=E_PENDING;
    HRESULT request=context->RequestEditSession(client,session,TF_ES_ASYNC|(write?TF_ES_READWRITE:TF_ES_READ),&outcome);
    auto deadline=GetTickCount64()+3000;
    while(SUCCEEDED(request)&&!session->completed&&GetTickCount64()<deadline){pump();Sleep(5);}
    if(SUCCEEDED(request)&&!session->completed){printf("pending_callback_deadline\n");fflush(stdout);ExitProcess(30);}
    return session->completed;
}
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount")return 2;
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int failures=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager)))||FAILED(manager->Activate(&client)))return 4;
        auto library=LoadLibraryExW(L"msctf.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
        using SetScope=HRESULT(WINAPI*)(HWND,InputScope);
        auto setScope=reinterpret_cast<SetScope>(GetProcAddress(library,"SetInputScope"));if(!setScope)return 5;
        for(int test=0;test<5;++test) {
            HWND top=CreateWindowW(L"STATIC",L"Grammar owned Edit scope",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,600,250,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
            HWND field=CreateWindowW(L"Edit",L"hello world",WS_CHILD|WS_VISIBLE,10,10,550,100,top,nullptr,GetModuleHandleW(nullptr),nullptr);
            ShowWindow(top,SW_SHOW);ShowWindow(top,SW_SHOW);SetForegroundWindow(top);SetFocus(field);
            if(test!=2&&FAILED(setScope(field,test==1?IS_PASSWORD:IS_TEXT)))return 6;
            SendMessageW(field,EM_SETSEL,6,11);
            auto until=GetTickCount64()+200;do{pump();Sleep(5);}while(GetTickCount64()<until);
            ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
            bool bound=SUCCEEDED(manager->GetFocus(&document))&&document&&SUCCEEDED(document->GetTop(&context))&&context;
            bool inSession=false,authority=true;TfEditCookie activeCookie=TF_INVALID_COOKIE;
            StandardEditBinding binding(field,[&]{return authority&&inSession&&standard_edit_plain_scope(context.Get(),activeCookie,field);});
            std::wstring selected;bool captured=false,scope=false;
            ComPtr<Session> capture;
            capture.Attach(new Session([&](TfEditCookie cookie){
                inSession=true;activeCookie=cookie;scope=standard_edit_plain_scope(context.Get(),cookie,field);
                captured=binding.capture(selected);inSession=false;
            }));
            bool ran=bound&&run(context.Get(),client,capture.Get(),false);
            if(test==3){setScope(field,IS_PASSWORD);pump();}
            if(test==4)authority=false;
            ApplyResult result=ApplyResult::rejected;
            ComPtr<Session> apply;
            apply.Attach(new Session([&](TfEditCookie cookie){inSession=true;activeCookie=cookie;result=binding.apply(L"Grammar");inSession=false;}));
            bool applied=bound&&run(context.Get(),client,apply.Get(),true);
            WCHAR text[128]{};GetWindowTextW(field,text,128);
            bool pass=ran&&applied&&(test==1||test==2?!captured&&binding.text_reads()==0:captured&&selected==L"world")&&
                (test==0?result==ApplyResult::applied&&std::wstring(text)==L"hello Grammar":result==ApplyResult::rejected&&std::wstring(text)==L"hello world")&&binding.close();
            printf("scope_case=%d pass=%d bound=%d scope=%d capture=%d reads=%u result=%u mutations=%u\n",test,pass,bound,scope,captured,binding.text_reads(),static_cast<unsigned>(result),binding.mutation_calls());
            if(!pass)++failures;
            DestroyWindow(top);pump();
        }
        manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("failures=%d\n",failures);return failures?1:0;
}
