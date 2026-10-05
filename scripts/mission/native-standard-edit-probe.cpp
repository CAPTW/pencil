// Candidate standard Edit production-class tests, owned restricted Sandbox only.
#include "../../src-tauri/native/standard_edit.hpp"
#include <cstdio>
using namespace grammar;
static int mode=0;
static bool color_edit=false;
static unsigned changes=0,updates=0;
static LRESULT CALLBACK parent_proc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp) {
    if(msg==WM_CTLCOLOREDIT && mode==3) {
        mode=0;color_edit=true;
        SetWindowTextW(reinterpret_cast<HWND>(lp),L"application owns this change");
    }
    if(msg==WM_COMMAND) {
        const auto code=HIWORD(wp);
        if(code==EN_CHANGE) ++changes;
        if(code==EN_UPDATE) ++updates;
        if((mode==1 && code==EN_UPDATE)||(mode==2 && code==EN_CHANGE)) {
            mode=0;
            SetWindowTextW(reinterpret_cast<HWND>(lp),L"application owns this change");
        }
    }
    return DefWindowProcW(hwnd,msg,wp,lp);
}
static WNDPROC prior=nullptr;
static LRESULT CALLBACK custom_proc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp) { return CallWindowProcW(prior,hwnd,msg,wp,lp); }
static void pump() { MSG m; while(PeekMessageW(&m,nullptr,0,0,PM_REMOVE)){TranslateMessage(&m);DispatchMessageW(&m);} }
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount") return 2;
    WNDCLASSW wc{};wc.lpfnWndProc=parent_proc;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"GrammarStandardEditOwned";
    if(!RegisterClassW(&wc)) return 3;
    int failures=0;
    for(int scenario=0;scenario<28;++scenario) {
        HWND top=CreateWindowW(wc.lpszClassName,L"Grammar standard Edit fixture",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,600,260,nullptr,nullptr,wc.hInstance,nullptr);
        HWND field=CreateWindowW(L"Edit",L"hello world",WS_CHILD|WS_VISIBLE|((scenario==19||scenario==21)?0:ES_MULTILINE),10,10,550,150,top,reinterpret_cast<HMENU>(1),wc.hInstance,nullptr);
        HWND other=CreateWindowW(L"Edit",L"untouched",WS_CHILD|WS_VISIBLE,10,170,550,30,top,reinterpret_cast<HMENU>(2),wc.hInstance,nullptr);
        ShowWindow(top,SW_SHOW);ShowWindow(top,SW_SHOW);SetForegroundWindow(top);SetFocus(field);SendMessageW(field,EM_SETSEL,6,11);pump();
        if(scenario==18||scenario==19) {
            mode=0;changes=updates=0;
            SendMessageW(field,EM_REPLACESEL,TRUE,reinterpret_cast<LPARAM>(L"Grammar"));
            WCHAR text[128]{};GetWindowTextW(field,text,128);bool replaced=std::wstring(text)==L"hello Grammar";
            auto undo=SendMessageW(field,EM_UNDO,0,0);GetWindowTextW(field,text,128);
            printf("unmodified_control_baseline case=%d replacement_match=%d undo_return=%lld undo_units=%zu undo_original_match=%d\n",scenario,replaced,static_cast<long long>(undo),wcslen(text),std::wstring(text)==L"hello world");
            DestroyWindow(top);pump();continue; // Diagnostic, not adapter acceptance.
        }
        if(scenario==3) SendMessageW(field,EM_SETPASSWORDCHAR,L'*',0);
        if(scenario==4) SendMessageW(field,EM_SETREADONLY,TRUE,0);
        if(scenario==12) prior=reinterpret_cast<WNDPROC>(SetWindowLongPtrW(field,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(custom_proc)));
        if(scenario==26)SetWindowLongPtrW(field,GWL_STYLE,GetWindowLongPtrW(field,GWL_STYLE)|ES_OEMCONVERT);
        bool pass=false;
        auto lifetime=std::make_shared<int>(0);std::weak_ptr<void> callbacks=lifetime;
        {
            bool authority=true;
            StandardEditBinding binding(field,[&]{return authority;},lifetime);
            lifetime.reset();
            std::wstring selected;
            bool captured=binding.capture(selected);
            const bool denied=scenario==3||scenario==4||scenario==12||scenario==26;
            pass=denied?!captured&&binding.text_reads()==0:captured&&selected==L"world";
            mode=0;changes=updates=0;color_edit=false;
            if(scenario==1) SetWindowTextW(field,L"application changed before Accept");
            if(scenario==2) SendMessageW(field,EM_SETSEL,0,5);
            if(scenario==5) {SetWindowTextW(field,L"transient");SetWindowTextW(field,L"hello world");SendMessageW(field,EM_SETSEL,6,11);}
            if(scenario==6) {SendMessageW(field,EM_SETSEL,0,5);SendMessageW(field,EM_SETSEL,6,11);}
            if(scenario==7) binding.cancel();
            if(scenario==8) mode=1;
            if(scenario==9) mode=2;
            if(scenario==10) SendMessageW(field,EM_SETPASSWORDCHAR,L'*',0);
            if(scenario==11) SendMessageW(field,EM_SETREADONLY,TRUE,0);
            if(scenario==13) {SetFocus(other);SetFocus(field);}
            if(scenario==14) SendMessageW(field,WM_IME_STARTCOMPOSITION,0,0);
            if(scenario==15) SendMessageW(field,EM_SETLIMITTEXT,12,0);
            if(scenario==16) {DestroyWindow(field);field=nullptr;}
            if(scenario==20) mode=3;
            if(scenario==22) authority=false;
            if(scenario==23||scenario==25) prior=reinterpret_cast<WNDPROC>(SetWindowLongPtrW(field,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(custom_proc)));
            if(scenario==24) { SetWindowLongPtrW(field,GWL_STYLE,GetWindowLongPtrW(field,GWL_STYLE)|ES_UPPERCASE); }
            if(scenario==27)SendMessageW(field,WM_MOUSEMOVE,MK_LBUTTON,0);
            auto result=binding.apply(scenario==17?L"Grammar \xd55c\xae00 \xd83d\xde00":L"Grammar");
            WCHAR actual[256]{};if(field)GetWindowTextW(field,actual,256);
            const bool normal=scenario==0||scenario==17||scenario==21||(scenario==20&&!color_edit), during=scenario==8||scenario==9||(scenario==20&&color_edit);
            const std::wstring expected=normal?(scenario==17?L"hello Grammar \xd55c\xae00 \xd83d\xde00":L"hello Grammar"):
                during?L"application owns this change":scenario==1?L"application changed before Accept":scenario==16?L"":L"hello world";
            pass=pass&&std::wstring(actual)==expected&&binding.mutation_calls()==(normal||during?1u:0u)&&
                result==(normal?ApplyResult::applied:during?ApplyResult::mutation_unconfirmed:ApplyResult::rejected);
            if(normal && scenario!=20) {
                printf("before_undo case=%d pass=%d units=%zu can_undo=%lld\n",scenario,pass,wcslen(actual),static_cast<long long>(SendMessageW(field,EM_CANUNDO,0,0)));
                pass=pass&&binding.apply(L"replay")==ApplyResult::rejected;
                auto undone=SendMessageW(field,EM_UNDO,0,0);GetWindowTextW(field,actual,256);
                printf("undo case=%d return=%lld units=%zu original_match=%d\n",scenario,static_cast<long long>(undone),wcslen(actual),std::wstring(actual)==L"hello world");
                pass=pass&&std::wstring(actual)==L"hello world";
            }
            if(scenario==20) {
                // Re-enable normal painting and actually exercise the app callback.
                UpdateWindow(field); pump(); GetWindowTextW(field,actual,256);
                bool afterPaint=color_edit&&std::wstring(actual)==L"application owns this change";
                printf("post_paint_app_edit_preserved=%d\n",afterPaint); pass=pass&&afterPaint;
            }
            if(scenario==23) {
                pass=pass&&!binding.close();
                SetWindowLongPtrW(field,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(prior));
            }
            pass=pass&&(scenario==25?!binding.close():binding.close());
            printf("case=%d pass=%d capture=%d reads=%u mutations=%u result=%u deferred=%u changes=%u updates=%u\n",scenario,pass,captured,binding.text_reads(),binding.mutation_calls(),static_cast<unsigned>(result),binding.deferred_notices(),changes,updates);
            if(scenario==20) printf("color_callback_edit=%d\n",color_edit);
        }
        if(scenario==25)pass=pass&&!callbacks.expired();
        if(scenario==12) SetWindowLongPtrW(field,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(prior));
        DestroyWindow(top);pump();
        pass=pass&&callbacks.expired();
        if(!pass)++failures;
    }
    UnregisterClassW(wc.lpszClassName,wc.hInstance);printf("failures=%d\n",failures);return failures?1:0;
}
