#pragma once
#include "word_dispatcher.hpp"
#include "apply_channel.hpp"

namespace grammar {
// Target-STA route for an already authenticated channel. Transport supplies a
// kernel-backed lease, never an arbitrary boolean from a wire request. The
// native target bootstrap still rejects Word until real-provider qualification.
class WordConnectionDispatcher final {
    const DWORD thread_ = GetCurrentThreadId();
    WordTargetDispatcher actor_;
    std::shared_ptr<ApplyConnectionLease> lease_;
    bool entering_ = false;
    bool retiring_ = false;
    struct Entry {
        bool& value;
        explicit Entry(bool& flag):value(flag){value=true;}
        ~Entry(){value=false;}
    };
    bool owner() const { return GetCurrentThreadId()==thread_; }
public:
    WordConnectionDispatcher(TfClientId client,std::shared_ptr<void> lifetime)
        :actor_(client,std::move(lifetime)){}
    bool pending() const { return entering_ || actor_.pending(); }
    void cancel() { if(owner())actor_.cancel(); }
    bool cancel_capture(uint64_t id) { return owner() && actor_.cancel_capture(id); }
    bool close() {
        if(!owner())return false;
        retiring_=true;actor_.cancel();
        if(entering_)return false;
        Entry entry(entering_);
        if(!actor_.close())return false;
        lease_.reset();retiring_=false;return true;
    }
    bool begin_capture(ITfContext* context,HWND field,HWND root,
        std::shared_ptr<WordNativeWindowPort> port,bool optIn,
        std::shared_ptr<ApplyConnectionLease> lease,std::chrono::steady_clock::time_point deadline,ULONGLONG deadlineTicks=MAXULONGLONG) {
        if(!owner() || entering_ || actor_.pending() || !lease || !lease->alive() || !optIn ||
            GetTickCount64()>=deadlineTicks || std::chrono::steady_clock::now()>=deadline)return false;
        // An unresolved event subscription retains the old lease and actor;
        // callers cannot start another channel while that ownership is live.
        if(!close())return false;
        Entry entry(entering_);
        lease_=std::move(lease);
        std::weak_ptr<ApplyConnectionLease> weak=lease_;
        const bool admitted=actor_.begin_capture(context,field,root,std::move(port),true,[weak]{
            auto connection=weak.lock();return connection && connection->alive();
        },deadline,deadlineTicks);
        if(retiring_){actor_.cancel();return false;}
        return admitted;
    }
    WordDispatchStatus capture_status() {
        if(!owner() || entering_ || retiring_)return WordDispatchStatus::rejected;
        Entry entry(entering_);return actor_.capture_status();
    }
    uint64_t capture_id() const { return owner() && !retiring_?actor_.capture_id():0; }
    bool selected_text(uint64_t id,std::wstring& text) {
        text.clear();if(!owner() || entering_ || retiring_)return false;
        Entry entry(entering_);return actor_.selected_text(id,text);
    }
    bool begin_accept(uint64_t id,std::wstring replacement,std::chrono::steady_clock::time_point deadline,ULONGLONG deadlineTicks=MAXULONGLONG) {
        if(!owner() || entering_ || retiring_ || !lease_ || !lease_->alive())return false;
        Entry entry(entering_);return actor_.begin_accept(id,std::move(replacement),deadline,deadlineTicks);
    }
    bool can_apply(uint64_t id) {
        return owner() && !entering_ && !retiring_ && !actor_.pending() && id &&
            id==actor_.capture_id() && capture_status()==WordDispatchStatus::captured;
    }
    bool with_activation(uint64_t id,const std::function<bool()>& activate) {
        if(!can_apply(id) || !activate)return false;
        Entry entry(entering_);
        if(!activate() || retiring_ || id!=actor_.capture_id())return false;
        return actor_.capture_status()==WordDispatchStatus::captured;
    }
    WordDispatchStatus accept_status() const { return owner()?actor_.accept_status():WordDispatchStatus::rejected; }
    unsigned mutations() const { return owner()?actor_.mutations():0; }
    // Caller must retain this route until checked close succeeds. A dropped
    // channel revokes mutation immediately; cleanup completion is separate.
    ~WordConnectionDispatcher(){close();}
};
}
