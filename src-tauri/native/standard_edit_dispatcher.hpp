#pragma once
#include "standard_edit.hpp"
#include "standard_edit_scope.hpp"
#include "tsf_dispatcher.hpp"

namespace grammar {
// Candidate only until the authenticated native service and installed widget
// qualify it. Scope metadata is read under a real TSF cookie; text belongs to
// the standard Edit control and is never taken from TSF's empty proxy text.
class StandardEditDispatcher final {
    struct State {
        ComPtr<ITfContext> context;
        ComPtr<ITfSource> subscription;
        DWORD subscription_cookie=TF_INVALID_COOKIE;
        std::shared_ptr<void> lifetime;
        std::shared_ptr<ApplyConnectionLease> lease;
        std::unique_ptr<StandardEditBinding> binding;
        HWND field=nullptr;
        bool enabled=true, in_session=false;
        TfEditCookie cookie=TF_INVALID_COOKIE;
        std::chrono::steady_clock::time_point active_deadline{};
        ULONGLONG active_deadline_ticks=0;
        StandardScopeIdentity scope;
        bool activating=false,scope_pending=false;
        ~State() {
            enabled=false;
            if(binding) binding->cancel();
            if(subscription&&subscription_cookie!=TF_INVALID_COOKIE)subscription->UnadviseSink(subscription_cookie);
        }
        bool authority() const {
            StandardScopeIdentity current;
            return enabled&&lease&&lease->alive()&&in_session&&!activating&&!scope_pending&&
                GetTickCount64()<active_deadline_ticks&&std::chrono::steady_clock::now()<active_deadline&&
                standard_edit_plain_scope(context.Get(),cookie,field,&current)&&current==scope&&
                GetTickCount64()<active_deadline_ticks&&std::chrono::steady_clock::now()<active_deadline;
        }
    };
    class Sink final : public ITfTextEditSink {
        LONG refs_=1; std::weak_ptr<State> state_;
    public:
        explicit Sink(std::weak_ptr<State> state):state_(std::move(state)) {}
        HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
            if(!out)return E_POINTER;*out=nullptr;
            if(id!=IID_IUnknown&&id!=IID_ITfTextEditSink)return E_NOINTERFACE;
            *out=static_cast<ITfTextEditSink*>(this);AddRef();return S_OK;
        }
        ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs_);}
        ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs_);if(!n)delete this;return n;}
        HRESULT STDMETHODCALLTYPE OnEndEdit(ITfContext*,TfEditCookie cookie,ITfEditRecord* record) override {
            // Standard Edit's TSF proxy text is not the Edit document. Native
            // window messages/EN_CHANGE track its text and selection revision.
            // This sink tracks scope transitions (including password -> text
            // ABA), not activation changes to the proxy's separate text store.
            const GUID* properties[]={&input_scope_property};
            ComPtr<IEnumTfRanges> ranges;ComPtr<ITfRange> changed;ULONG count=0;
            bool revoke=!record||FAILED(record->GetTextAndPropertyUpdates(0,properties,1,&ranges))||!ranges||
                FAILED(ranges->Next(1,&changed,&count));
            auto state=state_.lock();
            if(!state)return S_OK;
            if(count) {
                // Activation republishes the same scope. Compare its exact
                // non-sensitive enum list; missing/different scope revokes.
                // This never reads proxy or document text. A password update
                // invalidates immediately and cannot be undone by a later scope.
                StandardScopeIdentity current;
                bool absent=false;
                bool plain=standard_edit_scope_value(state->context.Get(),cookie,&current,&absent);
                bool same=plain&&current==state->scope;
                GRAMMAR_EDIT_DIAG("standard_scope_republication identical=%d plain=%d size=%u first=%d captured_first=%d\n",same,plain,current.size,current.size?static_cast<int>(current.values[0]):-1,state->scope.size?static_cast<int>(state->scope.values[0]):-1);
                if(state->activating&&absent)state->scope_pending=true;
                else if(same)state->scope_pending=false;
                else revoke=true;
            }
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
            ComPtr<IEnumTfRanges> proxyRanges;ComPtr<ITfRange> proxyChanged;ULONG proxyCount=0;
            if(record&&SUCCEEDED(record->GetTextAndPropertyUpdates(TF_GTP_INCL_TEXT,nullptr,0,&proxyRanges))&&proxyRanges)proxyRanges->Next(1,&proxyChanged,&proxyCount);
            GRAMMAR_EDIT_DIAG("standard_tsf_updates revoke=%d scope_count=%lu proxy_text_count=%lu\n",revoke,count,proxyCount);
#endif
            if(revoke&&state->binding)state->binding->cancel(0x10000002);
            return S_OK;
        }
    };
    class Session final : public ITfEditSession {
        LONG refs_=1;
        std::shared_ptr<State> state_;
        std::wstring replacement_;
        bool write_;
        const std::chrono::steady_clock::time_point deadline_;
        const ULONGLONG deadline_ticks_;
    public:
        bool called=false,completed=false;
        DispatchStatus status=DispatchStatus::pending;
        std::wstring selected;
        Session(std::shared_ptr<State> state,bool write,std::wstring replacement,std::chrono::steady_clock::time_point deadline,ULONGLONG deadlineTicks):state_(std::move(state)),replacement_(std::move(replacement)),write_(write),deadline_(deadline),deadline_ticks_(deadlineTicks) {}
        ~Session(){clear_text();if(!replacement_.empty())SecureZeroMemory(replacement_.data(),replacement_.size()*sizeof(wchar_t));}
        void clear_text(){if(!selected.empty())SecureZeroMemory(selected.data(),selected.size()*sizeof(wchar_t));selected.clear();}
        HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
            if(!out)return E_POINTER;*out=nullptr;
            if(id!=IID_IUnknown&&id!=IID_ITfEditSession)return E_NOINTERFACE;
            *out=static_cast<ITfEditSession*>(this);AddRef();return S_OK;
        }
        ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs_);}
        ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs_);if(!n)delete this;return n;}
        HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
            if(called)return E_UNEXPECTED;
            called=true;status=DispatchStatus::rejected;
            struct Complete { Session& self; ~Complete(){self.state_->in_session=false;self.completed=true;} } complete{*this};
            try {
                if(!state_->enabled||std::chrono::steady_clock::now()>=deadline_||GetTickCount64()>=deadline_ticks_)return S_FALSE;
                state_->in_session=true;state_->cookie=cookie;state_->active_deadline=deadline_;state_->active_deadline_ticks=deadline_ticks_;
                if(!write_&&!standard_edit_plain_scope(state_->context.Get(),cookie,state_->field,&state_->scope))return S_FALSE;
                if(!state_->authority()){GRAMMAR_EDIT_DIAG("standard_session authority_false write=%d\n",write_);return S_FALSE;}
                if(!write_) {
                    if(state_->binding->capture(selected))status=DispatchStatus::captured;
                } else {
                    // Any exception after entry may follow a mutation. Never
                    // describe an uncertain result as a rejection with no write.
                    status=DispatchStatus::mutation_unconfirmed;
                    switch(state_->binding->apply(replacement_)) {
                    case ApplyResult::applied:status=DispatchStatus::applied;break;
                    case ApplyResult::rejected:status=DispatchStatus::rejected;break;
                    default:break;
                    }
                }
                return S_OK;
            }catch(...){return E_FAIL;}
        }
    };
    DWORD thread_=GetCurrentThreadId();
    TfClientId client_;
    std::shared_ptr<void> lifetime_;
    std::shared_ptr<State> state_;
    ComPtr<Session> capture_,apply_;
    uint64_t id_=0,next_=1;
    bool consumed_=true;
    bool on_thread()const{return GetCurrentThreadId()==thread_;}
    bool pending()const{return(capture_&&!capture_->completed)||(apply_&&!apply_->completed);}
    bool request(Session* session,bool write) {
        HRESULT status=E_PENDING;
        HRESULT hr=state_->context->RequestEditSession(client_,session,TF_ES_ASYNC|(write?TF_ES_READWRITE:TF_ES_READ),&status);
        return session->completed||(SUCCEEDED(hr)&&status==TF_S_ASYNC);
    }
public:
    explicit StandardEditDispatcher(TfClientId client,std::shared_ptr<void> lifetime={}):client_(client),lifetime_(std::move(lifetime)) {}
    ~StandardEditDispatcher(){cancel();}
    void cancel(){if(!on_thread())return;consumed_=true;id_=0;if(state_){state_->enabled=false;if(state_->binding)state_->binding->cancel();}if(capture_)capture_->clear_text();}
    bool cancel_capture(uint64_t id){if(!on_thread()||!id||id!=id_)return false;cancel();return true;}
    bool executing()const{return(capture_&&capture_->called&&!capture_->completed)||(apply_&&apply_->called&&!apply_->completed);}
    uint64_t capture_id()const{return consumed_?0:id_;}
    bool begin_capture(ITfContext* context,HWND field,bool explicitOptIn,std::shared_ptr<ApplyConnectionLease> lease,
        std::chrono::steady_clock::time_point deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5),ULONGLONG deadlineTicks=MAXULONGLONG) {
        if(!on_thread()||pending()||(state_&&state_->activating)||!explicitOptIn||!lease||!lease->alive()||!context||next_==(std::numeric_limits<uint64_t>::max)()||std::chrono::steady_clock::now()>=deadline||GetTickCount64()>=deadlineTicks)return false;
        cancel();capture_.Reset();apply_.Reset();state_.reset();
        state_=std::make_shared<State>();state_->context=context;state_->field=field;state_->lease=std::move(lease);state_->lifetime=lifetime_;
        std::weak_ptr<State> weak=state_;
        state_->binding=std::make_unique<StandardEditBinding>(field,[weak]{auto state=weak.lock();return state&&state->authority();},lifetime_);
        ComPtr<Sink> sink;sink.Attach(new Sink(weak));
        if(FAILED(context->QueryInterface(IID_PPV_ARGS(&state_->subscription)))||
            FAILED(state_->subscription->AdviseSink(IID_ITfTextEditSink,sink.Get(),&state_->subscription_cookie))){cancel();return false;}
        capture_.Attach(new Session(state_,false,{},deadline,deadlineTicks));id_=next_++;consumed_=false;
        if(!request(capture_.Get(),false)){cancel();capture_.Reset();return false;}return true;
    }
    DispatchStatus capture_status(){
        if(!on_thread()||consumed_||!state_||!state_->enabled||!state_->lease->alive()||!capture_){cancel();return DispatchStatus::rejected;}
        if(!capture_->completed)return DispatchStatus::pending;
        if(capture_->status==DispatchStatus::captured&&!state_->binding->grant_alive()){cancel();return DispatchStatus::rejected;}
        return capture_->status;
    }
    bool selected_text(uint64_t id,std::wstring& text){text.clear();if(!id||id!=id_||capture_status()!=DispatchStatus::captured)return false;text=capture_->selected;return true;}
    bool can_apply(uint64_t id){return on_thread()&&!pending()&&id&&id==id_&&capture_status()==DispatchStatus::captured&&!state_->activating;}
    template<class Activate> bool with_activation(uint64_t id,Activate activate) {
        if(!can_apply(id))return false;
        {
            // Only a valid explicit Accept can enter this synchronous guard.
            // Standard Edit clears/re-publishes app scope during activation.
            // No capture/apply reads run in this interval. A known forbidden
            // scope always revokes; an absent scope must be restored exactly
            // before this guard ends, or the capture is permanently cancelled.
            struct Activation {
                State& state;
                explicit Activation(State& s):state(s){state.activating=true;state.scope_pending=false;}
                ~Activation(){state.activating=false;if(state.scope_pending)state.binding->cancel();}
            } activation{*state_};
            if(!activate())return false;
        }
        return can_apply(id);
    }
    bool begin_apply(uint64_t id,std::wstring replacement,std::chrono::steady_clock::time_point deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5),ULONGLONG deadlineTicks=MAXULONGLONG){
        if(!can_apply(id)||std::chrono::steady_clock::now()>=deadline||GetTickCount64()>=deadlineTicks)return false;
        apply_.Attach(new Session(state_,true,std::move(replacement),deadline,deadlineTicks));consumed_=true;capture_->clear_text();
        if(!request(apply_.Get(),true)){cancel();apply_.Reset();return false;}return true;
    }
    DispatchStatus apply_status()const{return on_thread()&&apply_?apply_->status:DispatchStatus::rejected;}
    void retire_completed_apply(){if(!on_thread()||!apply_||!apply_->completed)return;cancel();apply_.Reset();capture_.Reset();state_.reset();}
};
}
