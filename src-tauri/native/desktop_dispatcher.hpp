#pragma once
#include "standard_edit_dispatcher.hpp"
#include "target_apply_focus.hpp"
#include "word_connection.hpp"

namespace grammar {
// Dispatch by verified editor control, never by the opened file extension.
class DesktopTargetDispatcher final {
    std::unique_ptr<TsfTargetDispatcher> rich_;
    std::unique_ptr<StandardEditDispatcher> standard_;
    std::unique_ptr<WordConnectionDispatcher> word_;
    std::shared_ptr<WordNativeWindowPort> word_port_;
    HWND word_root_=nullptr,word_field_=nullptr;
    static DispatchStatus status(WordDispatchStatus value){
        switch(value){
        case WordDispatchStatus::pending:return DispatchStatus::pending;
        case WordDispatchStatus::captured:return DispatchStatus::captured;
        case WordDispatchStatus::applied:return DispatchStatus::applied;
        case WordDispatchStatus::mutation_unconfirmed:return DispatchStatus::mutation_unconfirmed;
        default:return DispatchStatus::rejected;
        }
    }
public:
    DesktopTargetDispatcher(TfClientId client,bool standard,std::shared_ptr<void> lifetime) {
        if(standard)standard_=std::make_unique<StandardEditDispatcher>(client,std::move(lifetime));
        else rich_=std::make_unique<TsfTargetDispatcher>(client,std::move(lifetime));
    }
    // Internal trusted-target route only. Bootstrap still admits Edit/RichEdit;
    // do not select this overload based on a wire value or file extension.
    DesktopTargetDispatcher(TfClientId client,HWND root,std::shared_ptr<WordNativeWindowPort> port,std::shared_ptr<void> lifetime)
        :word_(std::make_unique<WordConnectionDispatcher>(client,std::move(lifetime))),word_port_(std::move(port)),word_root_(root){}
    void cancel(){if(word_)word_->cancel();else if(standard_)standard_->cancel();else rich_->cancel();}
    bool cancel_capture(uint64_t id){return word_?word_->cancel_capture(id):standard_?standard_->cancel_capture(id):rich_->cancel_capture(id);}
    bool executing()const{return word_?word_->pending():standard_?standard_->executing():rich_->executing();}
    bool close(){cancel();return word_?word_->close():!executing();}
    uint64_t capture_id()const{return word_?word_->capture_id():standard_?standard_->capture_id():rich_->capture_id();}
    DispatchStatus capture_status(){return word_?status(word_->capture_status()):standard_?standard_->capture_status():rich_->capture_status();}
    DispatchStatus apply_status()const{return word_?status(word_->accept_status()):standard_?standard_->apply_status():rich_->apply_status();}
    bool selected_text(uint64_t id,std::wstring& text){return word_?word_->selected_text(id,text):standard_?standard_->selected_text(id,text):rich_->selected_text(id,text);}
    bool can_apply(uint64_t id){return word_?word_->can_apply(id):standard_?standard_->can_apply(id):rich_->can_apply(id);}
    bool activate(uint64_t id,HWND field,DWORD broker){
        if(word_)return field==word_field_ && GetAncestor(field,GA_ROOT)==word_root_ &&
            word_->with_activation(id,[&]{return activate_apply_target(field,broker);});
        return standard_?standard_->with_activation(id,[&]{return activate_apply_target(field,broker);}):
            rich_->can_apply(id)&&activate_apply_target(field,broker);
    }
    bool begin_capture(ITfContext* context,HWND field,bool optIn,std::shared_ptr<ApplyConnectionLease> lease){
        if(word_)return false; // No implicit fresh budget for Word requests.
        return standard_?standard_->begin_capture(context,field,optIn,std::move(lease)):rich_->begin_capture(context,field,optIn,std::move(lease));
    }
    bool begin_capture(ITfContext* context,HWND field,bool optIn,std::shared_ptr<ApplyConnectionLease> lease,std::chrono::steady_clock::time_point deadline,ULONGLONG deadlineTicks=MAXULONGLONG){
        if(!word_)return standard_?standard_->begin_capture(context,field,optIn,std::move(lease),deadline,deadlineTicks):rich_->begin_capture(context,field,optIn,std::move(lease),deadline,deadlineTicks);
        if(!word_root_ || !word_port_ || GetAncestor(field,GA_ROOT)!=word_root_)return false;
        const bool admitted=word_->begin_capture(context,field,word_root_,word_port_,optIn,std::move(lease),deadline,deadlineTicks);
        if(admitted)word_field_=field;
        return admitted;
    }
    bool begin_apply(uint64_t id,std::wstring replacement){if(word_)return false;return standard_?standard_->begin_apply(id,std::move(replacement)):rich_->begin_apply(id,std::move(replacement));}
    bool begin_apply(uint64_t id,std::wstring replacement,std::chrono::steady_clock::time_point deadline,ULONGLONG deadlineTicks=MAXULONGLONG){return word_?word_->begin_accept(id,std::move(replacement),deadline,deadlineTicks):standard_?standard_->begin_apply(id,std::move(replacement),deadline,deadlineTicks):rich_->begin_apply(id,std::move(replacement),deadline,deadlineTicks);}
    void retire_completed_apply(){if(word_){word_->close();}else if(standard_)standard_->retire_completed_apply();else rich_->retire_completed_apply();}
};
}
