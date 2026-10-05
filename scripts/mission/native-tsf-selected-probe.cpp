// Owned isolated RichEdit store, NOT Word/Office or installed-app qualification.
#include "../../src-tauri/native/tsf_selected_range.hpp"
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
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount"){puts("REFUSED Sandbox only");return 2;}
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int exitCode=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client)))return 4;
        const auto library=LoadLibraryW(L"Msftedit.dll");
        HWND top=CreateWindowExW(0,L"STATIC",L"Grammar owned selected TSF test",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,650,300,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        HWND edit=CreateWindowExW(0,L"RICHEDIT50W",L"",WS_CHILD|WS_VISIBLE|ES_MULTILINE|ES_AUTOVSCROLL,10,10,600,200,top,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!library || !top || !edit)return 5;
        SendMessageW(edit,EM_EXLIMITTEXT,0,200000);
        SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);
        SetForegroundWindow(top);SetFocus(edit);pump(400);
        ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
        if(FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context)return 6;
        for(unsigned scenario=0;scenario<19;++scenario) {
            const std::wstring selected=scenario>=17?std::wstring(scenario==17?8192:8193,L'w'):L"world \ud55c\uae00 \U0001f600";
            const std::wstring prefix=scenario==2?std::wstring(70000,L'a'):L"prefix ";
            const std::wstring suffix=L" suffix "+selected;
            std::wstring original=prefix+selected+suffix;
            const LONG start=static_cast<LONG>(prefix.size()),end=start+static_cast<LONG>(selected.size());
            SetWindowTextW(edit,original.c_str());SendMessageW(edit,EM_SETSEL,start,end);pump(30);
            auto grant=std::make_shared<SelectedRangeGrant>();grant->field_eligible=true;grant->revision=7;
            grant->deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5);
            unsigned bindings=0;bool applying=false;
            grant->verify_binding=[&](ITfContext* current,TfEditCookie){
                ++bindings;
                if(applying && scenario==13)grant->valid=false;
                if(applying && scenario==14)++grant->revision;
                return current==context.Get() && scenario!=12;
            };
            if(scenario==11)grant->field_eligible=false;
            unsigned reads=0,mutations=0;std::unique_ptr<TsfSelectedSnapshot> snapshot;
            bool ok=run(context.Get(),client,TF_ES_READ,[&](TfEditCookie cookie){snapshot=TsfSelectedSnapshot::capture(context.Get(),cookie,7,grant,reads);return S_OK;});
            if(scenario==11 || scenario==12 || scenario==18) {
                ok=ok && !snapshot && (scenario==18?reads<=33:reads==0) && readFixture(edit)==original;
            } else {
                ok=ok && snapshot && snapshot->source()==selected;
                if(snapshot) {
                    if(scenario==3){original=prefix+L"other source"+suffix;SetWindowTextW(edit,original.c_str());SendMessageW(edit,EM_SETSEL,start,end);}
                    if(scenario==4)SendMessageW(edit,EM_SETSEL,end+8,end+8+selected.size());
                    if(scenario==5)grant->valid=false;
                    if(scenario==6)++grant->revision;
                    if(scenario==7)grant->deadline=std::chrono::steady_clock::now()-std::chrono::seconds(1);
                    if(scenario==8)SendMessageW(edit,EM_SETREADONLY,TRUE,0);
                    std::wstring replacement=scenario==1?L"x":L"reviewed \ud55c\uae00 \U0001f680 replacement";
                    if(scenario==9)replacement=std::wstring(8193,L'x');
                    if(scenario==10)replacement=std::wstring(1,static_cast<WCHAR>(0xd800));
                    if(scenario==15)replacement=L"bad\ufffcobject";
                    if(scenario==16)replacement=std::wstring(L"a\0b",3);
                    applying=true;ApplyResult outcome=ApplyResult::pending;
                    HRESULT disposition=E_PENDING;
                    const bool ran=run(context.Get(),client,TF_ES_READWRITE,[&](TfEditCookie cookie){outcome=snapshot->apply(cookie,replacement,reads,mutations);return S_OK;},&disposition);
                    if(scenario==8) {
                        // RichEdit refuses the request before calling our code.
                        // Explicitly retire that rejected request's grant.
                        ok=ok && !ran && disposition==TF_E_READONLY && outcome==ApplyResult::pending && mutations==0;
                        snapshot->revoke();outcome=ApplyResult::rejected;
                    } else ok=ok && ran;
                    const bool positive=scenario<3 || scenario==17;
                    const auto expected=positive?prefix+replacement+suffix:original;
                    ok=ok && readFixture(edit)==expected && outcome==(positive?ApplyResult::applied:ApplyResult::rejected) && mutations==(positive?1u:0u);
                    if(scenario==8)SendMessageW(edit,EM_SETREADONLY,FALSE,0);
                    const auto before=mutations;
                    ok=ok && run(context.Get(),client,TF_ES_READWRITE,[&](TfEditCookie cookie){return snapshot->apply(cookie,L"replay",reads,mutations)==ApplyResult::rejected?S_OK:E_FAIL;}) && before==mutations && readFixture(edit)==expected;
                    printf("case=%u outcome=%d ",scenario,static_cast<int>(outcome));
                }
            }
            printf("check=%u pass=%d reads=%u mutations=%u bindings=%u\n",scenario,ok,reads,mutations,bindings);
            if(!ok)exitCode=1;
        }
        context.Reset();document.Reset();DestroyWindow(top);manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("result=%s Office=NOT_RUN scope=isolated_RichEdit_transaction\n",exitCode?"FAIL":"PASS");return exitCode;
}
