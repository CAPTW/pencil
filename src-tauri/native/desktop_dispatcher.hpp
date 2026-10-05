#pragma once
#include "standard_edit_dispatcher.hpp"
#include "target_apply_focus.hpp"

namespace grammar {
// Dispatch by verified editor control, never by the opened file extension.
class DesktopTargetDispatcher final {
    std::unique_ptr<TsfTargetDispatcher> rich_;
    std::unique_ptr<StandardEditDispatcher> standard_;
public:
    DesktopTargetDispatcher(TfClientId client,bool standard,std::shared_ptr<void> lifetime) {
        if(standard)standard_=std::make_unique<StandardEditDispatcher>(client,std::move(lifetime));
        else rich_=std::make_unique<TsfTargetDispatcher>(client,std::move(lifetime));
    }
    void cancel(){if(standard_)standard_->cancel();else rich_->cancel();}
    bool cancel_capture(uint64_t id){return standard_?standard_->cancel_capture(id):rich_->cancel_capture(id);}
    bool executing()const{return standard_?standard_->executing():rich_->executing();}
    uint64_t capture_id()const{return standard_?standard_->capture_id():rich_->capture_id();}
    DispatchStatus capture_status(){return standard_?standard_->capture_status():rich_->capture_status();}
    DispatchStatus apply_status()const{return standard_?standard_->apply_status():rich_->apply_status();}
    bool selected_text(uint64_t id,std::wstring& text){return standard_?standard_->selected_text(id,text):rich_->selected_text(id,text);}
    bool can_apply(uint64_t id){return standard_?standard_->can_apply(id):rich_->can_apply(id);}
    bool activate(uint64_t id,HWND field,DWORD broker){
        return standard_?standard_->with_activation(id,[&]{return activate_apply_target(field,broker);}):
            rich_->can_apply(id)&&activate_apply_target(field,broker);
    }
    bool begin_capture(ITfContext* context,HWND field,bool optIn,std::shared_ptr<ApplyConnectionLease> lease){
        return standard_?standard_->begin_capture(context,field,optIn,std::move(lease)):rich_->begin_capture(context,field,optIn,std::move(lease));
    }
    bool begin_apply(uint64_t id,std::wstring replacement){return standard_?standard_->begin_apply(id,std::move(replacement)):rich_->begin_apply(id,std::move(replacement));}
    void retire_completed_apply(){if(standard_)standard_->retire_completed_apply();else rich_->retire_completed_apply();}
};
}
