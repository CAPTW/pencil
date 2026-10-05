#pragma once
// Target-thread standard Edit candidate. Broker admits single-line controls and
// multiline controls with pre-existing ES_AUTOVSCROLL; it never changes styles.
// No external verify/replace sequence, clipboard paste, readonly lock or rollback.
#include "tsf_apply.hpp"
#include <imm.h>
#include <array>
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
#include <cstdio>
#define GRAMMAR_EDIT_DIAG(...) do {printf(__VA_ARGS__);fflush(stdout);} while(false)
#else
#define GRAMMAR_EDIT_DIAG(...) do {} while(false)
#endif

namespace grammar {
class StandardEditBinding final {
    struct State;
    struct Hook { std::shared_ptr<State> state; WNDPROC previous; bool parent; };
    struct Notice { WPARAM wp; LPARAM lp; };
    struct State {
        HWND field = nullptr, parent = nullptr;
        DWORD thread = GetCurrentThreadId();
        WNDPROC base = nullptr;
        bool valid = false, composing = false, writing = false, overflow = false;
        uint64_t revision = 0, captured_revision = 0;
        DWORD start = 0, end = 0;
        std::wstring source;
        std::array<Notice,8> notices{};
        size_t notice_count = 0;
        std::chrono::steady_clock::time_point expires;
        std::shared_ptr<void> callback_lifetime;
        std::function<bool()> authority;
        unsigned reads = 0, mutations = 0, deferred = 0;
        void revoke(unsigned reason=0) { (void)reason; if(valid)GRAMMAR_EDIT_DIAG("standard_revoke reason=%u revision=%llu\n",reason,static_cast<unsigned long long>(revision)); valid = false; ++revision; }
        void clear() { if(!source.empty()) SecureZeroMemory(source.data(),source.size()*sizeof(wchar_t)); source.clear(); }
        ~State() { clear(); }
    };
    inline static constexpr WCHAR property[] = L"Grammar.StandardEdit.Owner.42e795a7";
    std::shared_ptr<State> state_;
    static bool changing(UINT msg) {
        switch(msg) {
        case WM_SETTEXT: case EM_REPLACESEL: case EM_SETSEL: case EM_SETHANDLE:
        case WM_CLEAR: case WM_CUT: case WM_PASTE: case WM_UNDO: case EM_UNDO:
        case EM_SETPASSWORDCHAR: case EM_SETREADONLY: case EM_SETLIMITTEXT:
        case WM_STYLECHANGING: case WM_NCDESTROY: case WM_KILLFOCUS:
        case WM_CHAR: case WM_KEYDOWN: case WM_LBUTTONDOWN: case WM_LBUTTONDBLCLK:
        case WM_RBUTTONDOWN: case WM_RBUTTONDBLCLK: case WM_POINTERDOWN: case WM_POINTERUPDATE:
        case WM_IME_STARTCOMPOSITION: case WM_IME_COMPOSITION: case WM_IME_ENDCOMPOSITION:
            return true;
        default: return false;
        }
    }
    static LRESULT CALLBACK procedure(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp) {
        auto hook=reinterpret_cast<Hook*>(GetPropW(hwnd,property));
        if(!hook) return DefWindowProcW(hwnd,msg,wp,lp);
        auto state=hook->state; auto previous=hook->previous; const bool parent=hook->parent;
        if(!parent) {
            // Observe invalidation only, never record key values or input history.
            if(changing(msg)||(msg==WM_MOUSEMOVE&&(wp&MK_LBUTTON))) {
                DWORD next=0;
                if(msg!=WM_KILLFOCUS || (wp && GetWindowThreadProcessId(reinterpret_cast<HWND>(wp),&next) && next==GetCurrentProcessId())) state->revoke(msg);
            }
            if(msg==WM_IME_STARTCOMPOSITION) state->composing=true;
            if(msg==WM_IME_ENDCOMPOSITION) state->composing=false;
        } else if(state->writing && msg==WM_COMMAND && reinterpret_cast<HWND>(lp)==state->field) {
            // Standard Edit sends synchronous parent notifications from inside
            // replacement. Deliver them afterwards so app callbacks cannot move
            // the selection mid-replacement. App changes during delivery win;
            // they are never overwritten by a Grammar restoration/retry.
            if(state->notice_count < state->notices.size()) state->notices[state->notice_count++]={wp,lp};
            else state->overflow=true;
            ++state->deferred;
            return 0;
        }
        if(parent && msg==WM_COMMAND && reinterpret_cast<HWND>(lp)==state->field && HIWORD(wp)==EN_CHANGE) state->revoke(0x10000001);
        if(msg==WM_NCDESTROY) {
            state->revoke();
            if(parent) state->parent=nullptr; else state->field=nullptr;
            RemovePropW(hwnd,property);
            if(reinterpret_cast<WNDPROC>(GetWindowLongPtrW(hwnd,GWLP_WNDPROC))==procedure)
                SetWindowLongPtrW(hwnd,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(previous));
            delete hook;
        }
        return CallWindowProcW(previous,hwnd,msg,wp,lp);
    }
    static bool install(HWND window,const std::shared_ptr<State>& state,bool parent) {
        if(GetPropW(window,property)) return false;
        auto hook=std::make_unique<Hook>(); hook->state=state; hook->parent=parent;
        hook->previous=reinterpret_cast<WNDPROC>(GetWindowLongPtrW(window,GWLP_WNDPROC));
        if(!hook->previous || !SetPropW(window,property,reinterpret_cast<HANDLE>(hook.get()))) return false;
        SetLastError(0);
        auto old=SetWindowLongPtrW(window,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(procedure));
        if(!old && GetLastError()) { RemovePropW(window,property); return false; }
        hook.release(); return true;
    }
    static bool owned(HWND window,const std::shared_ptr<State>& state) {
        if(!window || !IsWindow(window)) return false;
        auto hook=reinterpret_cast<Hook*>(GetPropW(window,property));
        return hook && hook->state==state && reinterpret_cast<WNDPROC>(GetWindowLongPtrW(window,GWLP_WNDPROC))==procedure;
    }
    static bool remove(HWND window) {
        auto hook=reinterpret_cast<Hook*>(GetPropW(window,property));
        SetLastError(0);
        if(!SetWindowLongPtrW(window,GWLP_WNDPROC,reinterpret_cast<LONG_PTR>(hook->previous))&&GetLastError())return false;
        RemovePropW(window,property); delete hook;
        return true;
    }
    static bool boundary(const std::wstring& text,DWORD pos) {
        return pos<=text.size() && (pos==0 || pos==text.size() ||
            !(text[pos-1]>=0xd800 && text[pos-1]<=0xdbff && text[pos]>=0xdc00 && text[pos]<=0xdfff));
    }
    bool eligible(bool redrawSuspended=false) const {
        auto& s=*state_; DWORD pid=0;
        if(!s.authority || !s.authority()) {GRAMMAR_EDIT_DIAG("standard_eligible authority_false\n");return false;}
        if(GetCurrentThreadId()!=s.thread || !owned(s.field,state_) || !owned(s.parent,state_) ||
            GetWindowThreadProcessId(s.field,&pid)!=s.thread || pid!=GetCurrentProcessId() ||
            GetParent(s.field)!=s.parent || !IsWindowUnicode(s.field) || (!redrawSuspended && !IsWindowVisible(s.field)) || !IsWindowVisible(s.parent) ||
            !IsWindowEnabled(s.field) || !IsWindowEnabled(s.parent) || s.composing || GetFocus()!=s.field ||
            GetAncestor(s.field,GA_ROOT)!=GetForegroundWindow()) return false;
        auto style=GetWindowLongPtrW(s.field,GWL_STYLE);
        if(style & (ES_PASSWORD|ES_READONLY|ES_NUMBER|ES_UPPERCASE|ES_LOWERCASE|ES_OEMCONVERT)) return false;
        if(CallWindowProcW(s.base,s.field,EM_GETPASSWORDCHAR,0,0)) return false;
        HIMC ime=ImmGetContext(s.field);
        if(ime) { LONG bytes=ImmGetCompositionStringW(ime,GCS_COMPSTR,nullptr,0); ImmReleaseContext(s.field,ime); if(bytes!=0) return false; }
        return true;
    }
    bool read(std::wstring& text,DWORD& start,DWORD& end) {
        auto& s=*state_; text.clear(); start=end=0;
        const auto size=CallWindowProcW(s.base,s.field,WM_GETTEXTLENGTH,0,0);
        if(size<0 || size>65534) return false;
        std::vector<WCHAR> buffer(static_cast<size_t>(size)+1);
        ++s.reads;
        auto count=CallWindowProcW(s.base,s.field,WM_GETTEXT,buffer.size(),reinterpret_cast<LPARAM>(buffer.data()));
        if(count!=size) { SecureZeroMemory(buffer.data(),buffer.size()*sizeof(WCHAR)); return false; }
        text.assign(buffer.data(),static_cast<size_t>(count)); SecureZeroMemory(buffer.data(),buffer.size()*sizeof(WCHAR));
        CallWindowProcW(s.base,s.field,EM_GETSEL,reinterpret_cast<WPARAM>(&start),reinterpret_cast<LPARAM>(&end));
        return text.find(L'\0')==std::wstring::npos && boundary(text,start) && boundary(text,end);
    }
public:
    explicit StandardEditBinding(HWND field,std::function<bool()> authority,std::shared_ptr<void> lifetime={}) : state_(std::make_shared<State>()) {
        auto& s=*state_; s.field=field; s.parent=GetParent(field); s.callback_lifetime=std::move(lifetime); s.authority=std::move(authority);
        DWORD pid=0; WNDCLASSEXW type{}; type.cbSize=sizeof(type); WCHAR name[32]{};
        if(!GetClassNameW(field,name,32) || _wcsicmp(name,L"Edit")!=0 || !IsWindowUnicode(field) ||
            GetWindowThreadProcessId(field,&pid)!=s.thread || pid!=GetCurrentProcessId() ||
            !s.parent || GetWindowThreadProcessId(s.parent,&pid)!=s.thread || pid!=GetCurrentProcessId() ||
            !GetClassInfoExW(nullptr,L"Edit",&type)) return;
        s.base=type.lpfnWndProc;
        // Existing custom subclasses are not silently bypassed.
        if(reinterpret_cast<WNDPROC>(GetWindowLongPtrW(field,GWLP_WNDPROC))!=s.base) return;
        if(!install(s.parent,state_,true)) return;
        if(!install(field,state_,false)) { remove(s.parent); return; }
    }
    ~StandardEditBinding() { cancel(); close(); }
    StandardEditBinding(const StandardEditBinding&)=delete;
    StandardEditBinding& operator=(const StandardEditBinding&)=delete;
    bool capture(std::wstring& selected) {
        auto& s=*state_; selected.clear(); cancel();
        if(!eligible()) return false;
        const auto revision=s.revision;
        if(!read(s.source,s.start,s.end) || s.start>=s.end || revision!=s.revision || !eligible()) { s.clear(); return false; }
        s.captured_revision=revision; s.expires=std::chrono::steady_clock::now()+std::chrono::seconds(30); s.valid=true;
        selected=s.source.substr(s.start,s.end-s.start); return true;
    }
    void cancel(unsigned reason=0) { state_->revoke(reason); state_->clear(); }
    bool grant_alive() const {
        const auto& s=*state_;
        return GetCurrentThreadId()==s.thread && s.valid && s.revision==s.captured_revision &&
            std::chrono::steady_clock::now()<s.expires && owned(s.field,state_) && owned(s.parent,state_);
    }
    bool close() {
        auto& s=*state_;
        if(GetCurrentThreadId()!=s.thread || s.writing) return false;
        // A new subclass may still call ours. Retain hook/callback ownership
        // until destruction rather than unloading its code beneath that chain.
        for(HWND window:{s.field,s.parent}) {
            if(window && IsWindow(window) && GetPropW(window,property) && !owned(window,state_)) return false;
        }
        for(HWND window:{s.field,s.parent}) if(window && IsWindow(window) && owned(window,state_) && !remove(window))return false;
        return true;
    }
    ApplyResult apply(const std::wstring& replacement) {
        auto& s=*state_;
        if(!s.valid || s.writing) return ApplyResult::rejected;
        s.valid=false;
        struct ClearSource { State& state; ~ClearSource() { state.clear(); } } clearSource{s};
        if(!eligible() || s.revision!=s.captured_revision || std::chrono::steady_clock::now()>=s.expires ||
            replacement.empty() || replacement.size()>65534 || replacement.find(L'\0')!=std::wstring::npos) return ApplyResult::rejected;
        std::wstring actual; DWORD start=0,end=0;
        if(!read(actual,start,end) || actual!=s.source || start!=s.start || end!=s.end || !eligible() ||
            s.revision!=s.captured_revision) return ApplyResult::rejected;
        const auto expected=s.source.substr(0,start)+replacement+s.source.substr(end);
        auto limit=CallWindowProcW(s.base,s.field,EM_GETLIMITTEXT,0,0);
        if(expected.size()>65534 || expected.size()>static_cast<size_t>(limit)) return ApplyResult::rejected;
        s.notice_count=0; s.overflow=false;
        bool replaced=false;
        {
            // Multiline Edit can call its parent's color/paint procedure between
            // deleting and inserting. Suspend only redraw for this synchronous
            // target-thread operation; never lock input or restore text.
            struct Redraw {
                State& s; HWND window; WNDPROC base;
                explicit Redraw(State& value):s(value),window(value.field),base(value.base) {
                    s.writing=true; CallWindowProcW(base,window,WM_SETREDRAW,FALSE,0);
                }
                ~Redraw() {
                    if(IsWindow(window)) { CallWindowProcW(base,window,WM_SETREDRAW,TRUE,0); InvalidateRect(window,nullptr,TRUE); }
                    s.writing=false;
                }
            } redraw(s);
            // Redraw admission must not itself authorize a changed source/range.
            if(eligible(true) && s.revision==s.captured_revision &&
                std::chrono::steady_clock::now()<s.expires &&
                read(actual,start,end) && actual==s.source && start==s.start && end==s.end &&
                eligible(true) && s.revision==s.captured_revision && std::chrono::steady_clock::now()<s.expires) {
                ++s.mutations; replaced=true;
                CallWindowProcW(s.base,s.field,EM_REPLACESEL,TRUE,reinterpret_cast<LPARAM>(replacement.c_str()));
            }
        }
        s.revoke(); s.clear();
        // Deliver each original notice once, in order, with no text snapshots.
        const auto notices=s.notices; const auto count=s.notice_count; s.notice_count=0;
        for(size_t i=0;i<count && s.parent && IsWindow(s.parent);++i)
            SendMessageW(s.parent,WM_COMMAND,notices[i].wp,notices[i].lp);
        if(!replaced) return ApplyResult::rejected;
        if(s.overflow || !eligible() || !read(actual,start,end) || actual!=expected) return ApplyResult::mutation_unconfirmed;
        return ApplyResult::applied;
    }
    unsigned text_reads() const { return state_->reads; }
    unsigned mutation_calls() const { return state_->mutations; }
    unsigned deferred_notices() const { return state_->deferred; }
};
}
