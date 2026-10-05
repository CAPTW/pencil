#pragma once
// Dedicated one-review desktop/native-host connection. The caller must also
// verify BrowserProcessRoute and the MV3 document/selection authority. This
// transport does not itself authorize any document read, inference or write.
#include "apply_peer.hpp"
#include "target_apply_protocol.hpp"
namespace grammar {
inline constexpr uint64_t browser_bridge_magic=0x31424752414d4d52ULL;
inline constexpr DWORD browser_bridge_units=8192;
inline std::wstring browser_bridge_name(DWORD host) {return target_pipe_name(0,host,0);}
enum class BrowserPoll { idle, request, closed };
class BrowserBridgeServer final {
    OwnedApplyPipe pipe_;
    std::wstring app_image_;
    ApplyImageDigest app_hash_;
    std::unique_ptr<VerifiedApplyPeer> peer_;
    std::unique_ptr<ApplyChannelAdmission> admission_;
    enum class Phase { fresh, capturing, ready, applying, done } phase_=Phase::fresh;
    ULONGLONG lifetime_=0,pending_deadline_=0;
    bool connected_=false,closed_=false;
    uint64_t capture_=0;
public:
    BrowserBridgeServer(const std::wstring& trustedApp,const ApplyImageDigest& trustedDigest)
        :pipe_(browser_bridge_name(GetCurrentProcessId())),app_image_(trustedApp),app_hash_(trustedDigest) {
        closed_=pipe_.get()==INVALID_HANDLE_VALUE;
    }
    bool available() const noexcept {return !closed_;}
    BrowserBridgeServer(const BrowserBridgeServer&)=delete;
    BrowserBridgeServer& operator=(const BrowserBridgeServer&)=delete;
    ~BrowserBridgeServer(){close();}
    void close() noexcept {
        closed_=true;phase_=Phase::done;capture_=0;
        if(admission_)admission_->revoke();
        if(connected_){DisconnectNamedPipe(pipe_.get());connected_=false;}
        admission_.reset();peer_.reset();
    }
    // Call from the single transport owner. No detached worker or pending
    // overlapped storage exists. Idle polling does not inspect document data.
    BrowserPoll poll(ChannelRequest& request) {
        request.clear();if(closed_)return BrowserPoll::closed;
        if(!connected_) {
            if(!ConnectNamedPipe(pipe_.get(),nullptr) && GetLastError()!=ERROR_PIPE_CONNECTED) {
                if(GetLastError()==ERROR_PIPE_LISTENING)return BrowserPoll::idle;
                close();return BrowserPoll::closed;
            }
            connected_=true;const auto now=GetTickCount64();lifetime_=now+35000;
            const auto deadline=now+3000;ULONG pid=0;
            if(!GetNamedPipeClientProcessId(pipe_.get(),&pid)) {close();return BrowserPoll::closed;}
            peer_=verify_apply_peer(pid,app_image_,app_hash_,deadline);
            if(!peer_) {close();return BrowserPoll::closed;}
            admission_=std::make_unique<ApplyChannelAdmission>(peer_->process());
            TargetHello hello;hello.magic=browser_bridge_magic;hello.server_pid=GetCurrentProcessId();
            hello.capability=admission_->bootstrap_capability();
            bool sent=admission_->peer_matches(pipe_.get()) && target_write(pipe_.get(),&hello,sizeof(hello),deadline);
            SecureZeroMemory(hello.capability.data(),hello.capability.size());
            if(!sent){close();return BrowserPoll::closed;}
        }
        const auto now=GetTickCount64();
        if(!admission_->peer_matches(pipe_.get()) || now>=lifetime_ || (pending_deadline_ && now>=pending_deadline_)) {
            close();return BrowserPoll::closed;
        }
        if(pending_deadline_)return BrowserPoll::idle;
        DWORD bytes=0;
        if(!PeekNamedPipe(pipe_.get(),nullptr,0,nullptr,&bytes,nullptr)){close();return BrowserPoll::closed;}
        if(!bytes)return BrowserPoll::idle;
        pending_deadline_=(std::min)(lifetime_,now+3000);
        if(!receive_apply_request(pipe_.get(),*admission_,request,pending_deadline_) ||
            request.header.payload_bytes>browser_bridge_units*sizeof(wchar_t)) {
            close();request.clear();return BrowserPoll::closed;
        }
        pending_deadline_=(std::min)(pending_deadline_,request.header.deadline_ticks);
        const auto kind=request.header.command;
        if(kind==ChannelCommand::capture && phase_==Phase::fresh)phase_=Phase::capturing;
        else if(kind==ChannelCommand::apply && phase_==Phase::ready && request.header.capture_id==capture_) {
            capture_=0;phase_=Phase::applying; // Consumed before forwarding to MV3.
        } else if(kind==ChannelCommand::cancel && phase_==Phase::ready && request.header.capture_id==capture_) {
            close();request.clear();return BrowserPoll::closed;
        } else {close();request.clear();return BrowserPoll::closed;}
        return BrowserPoll::request;
    }
    // status:0 rejected,2 captured,3 applied,4 indeterminate. Applied can only
    // come from the exact target's actual reread; never synthesize it on timeout.
    bool respond(uint32_t status,const std::wstring& source=L"") {
        if(closed_ || !pending_deadline_ || !admission_->peer_matches(pipe_.get()) ||
            GetTickCount64()>=pending_deadline_) {close();return false;}
        bool captured=phase_==Phase::capturing && status==2 && !source.empty() && source.size()<=browser_bridge_units;
        bool terminal=(phase_==Phase::capturing && status==0) ||
            (phase_==Phase::applying && (status==0 || status==3 || status==4));
        if((!captured && !terminal) || (!captured && !source.empty())){close();return false;}
        if(captured) {
            std::vector<unsigned char> bytes(source.size()*sizeof(wchar_t));
            memcpy(bytes.data(),source.data(),bytes.size());bool valid=channel_valid_text(bytes);
            SecureZeroMemory(bytes.data(),bytes.size());if(!valid){close();return false;}
        }
        TargetReply reply;reply.status=status;reply.units=static_cast<uint32_t>(source.size());reply.capture_id=captured?1:0;
        bool sent=target_write(pipe_.get(),&reply,sizeof(reply),pending_deadline_) &&
            target_write(pipe_.get(),source.data(),reply.units*sizeof(wchar_t),pending_deadline_);
        pending_deadline_=0;
        if(!sent){close();return false;}
        if(terminal){phase_=Phase::done;pending_deadline_=(std::min)(lifetime_,GetTickCount64()+3000);return true;}
        capture_=1;phase_=Phase::ready;return true;
    }
};
class BrowserBridgeClient final {
    std::unique_ptr<VerifiedApplyPeer> host_;
    ChannelHandle pipe_;
    ChannelCapability capability_{};
    std::atomic<bool> cancelled_{false};
    ULONGLONG lifetime_=0;
    uint64_t sequence_=1,capture_=0;
    bool connected_=false,captured_=false;
public:
    BrowserBridgeClient()=default;
    BrowserBridgeClient(const BrowserBridgeClient&)=delete;
    BrowserBridgeClient& operator=(const BrowserBridgeClient&)=delete;
    ~BrowserBridgeClient(){disconnect();}
    void cancel() noexcept {cancelled_=true;} // Other threads set only this flag.
    void disconnect() noexcept {connected_=false;capture_=0;pipe_.reset();host_.reset();SecureZeroMemory(capability_.data(),capability_.size());}
    bool connect(std::unique_ptr<VerifiedApplyPeer> trustedHost,ULONGLONG deadline) {
        if(host_ || captured_ || cancelled_ || !trustedHost || !trustedHost->alive() || GetTickCount64()>=deadline)return false;
        host_=std::move(trustedHost);lifetime_=GetTickCount64()+35000;
        const auto name=browser_bridge_name(GetProcessId(host_->process()));
        do {
            pipe_.reset(CreateFileW(name.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,
                SECURITY_SQOS_PRESENT|SECURITY_IDENTIFICATION,nullptr));
            if(pipe_.get()!=INVALID_HANDLE_VALUE)break;
            if(GetLastError()!=ERROR_FILE_NOT_FOUND && GetLastError()!=ERROR_PIPE_BUSY){disconnect();return false;}
            Sleep(1);
        } while(!cancelled_ && host_->alive() && GetTickCount64()<deadline);
        ULONG pid=0;DWORD mode=PIPE_NOWAIT;TargetHello hello;
        bool ok=pipe_.get()!=INVALID_HANDLE_VALUE && !cancelled_ && host_->alive() &&
            GetNamedPipeServerProcessId(pipe_.get(),&pid) && pid==GetProcessId(host_->process()) &&
            SetNamedPipeHandleState(pipe_.get(),&mode,nullptr,nullptr) &&
            channel_read_exact(pipe_.get(),&hello,sizeof(hello),deadline,&cancelled_) &&
            hello.magic==browser_bridge_magic && hello.version==1 && hello.server_pid==pid &&
            !hello.window && !hello.window_thread && !hello.worker_thread;
        if(ok){capability_=hello.capability;connected_=true;}
        SecureZeroMemory(hello.capability.data(),hello.capability.size());
        if(!ok)disconnect();return ok;
    }
    // False is a transport failure, not proof of no mutation. An Apply caller
    // must retain indeterminate status once transmission may have begun.
    bool request(ChannelCommand command,const std::wstring& replacement,TargetReply& reply,std::wstring& source,ULONGLONG outerDeadline=0) {
        reply={};reply.status=command==ChannelCommand::apply?4:0;source.clear();
        if(!connected_ || cancelled_ || !host_->alive() || GetTickCount64()>=lifetime_ ||
            replacement.size()>browser_bridge_units || sequence_==UINT64_MAX)return false;
        if(command==ChannelCommand::capture) {if(captured_ || !replacement.empty())return false;captured_=true;}
        else if(command==ChannelCommand::apply) {if(!capture_ || replacement.empty())return false;}
        else return false;
        ChannelHeader header;header.command=command;header.sequence=sequence_++;header.capture_id=capture_;
        header.capability=capability_;header.payload_bytes=static_cast<uint32_t>(replacement.size()*sizeof(wchar_t));
        if(command==ChannelCommand::apply)capture_=0;
        const auto deadline=(std::min)((std::min)(lifetime_,GetTickCount64()+3000),outerDeadline?outerDeadline:UINT64_MAX);
        header.deadline_ticks=deadline;
        TargetReply received;
        bool ok=target_write(pipe_.get(),&header,sizeof(header),deadline,&cancelled_) &&
            target_write(pipe_.get(),replacement.data(),header.payload_bytes,deadline,&cancelled_) &&
            channel_read_exact(pipe_.get(),&received,sizeof(received),deadline,&cancelled_);
        SecureZeroMemory(header.capability.data(),header.capability.size());
        if(ok)ok=command==ChannelCommand::capture ?
            ((received.status==2 && received.units>0 && received.units<=browser_bridge_units && received.capture_id==1) ||
             (received.status==0 && !received.units && !received.capture_id)) :
            ((received.status==0 || received.status==3 || received.status==4) && !received.units && !received.capture_id);
        if(ok && received.units) {
            std::vector<unsigned char> bytes(received.units*sizeof(wchar_t));
            ok=channel_read_exact(pipe_.get(),bytes.data(),static_cast<DWORD>(bytes.size()),deadline,&cancelled_) && channel_valid_text(bytes);
            if(ok){source.resize(received.units);memcpy(source.data(),bytes.data(),bytes.size());}
            SecureZeroMemory(bytes.data(),bytes.size());
        }
        if(ok){reply=received;capture_=received.capture_id;}
        if(!ok || command==ChannelCommand::apply || !capture_)disconnect();
        return ok;
    }
};
} // namespace grammar
