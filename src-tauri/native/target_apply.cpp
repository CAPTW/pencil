// Explicit-capture target bridge. No global/keyboard hook, profile registration,
// background document monitoring, Provider calls or persisted document content.
#include "desktop_dispatcher.hpp"
#include "apply_peer.hpp"
#include "target_apply_protocol.hpp"
#include "target_apply_focus.hpp"
#include <mutex>
#include <condition_variable>
#ifndef GRAMMAR_TRUSTED_APP_SHA256
#error Build the broker first and supply its trusted SHA256 through a build header.
#endif
#ifndef GRAMMAR_TRUSTED_APP_NAME
#error Supply the installed sibling broker filename through a build header.
#endif
using namespace grammar;
namespace {
std::atomic<bool> occupied{false}; // One connection per target process, including pending cleanup.
constexpr UINT wake_message = WM_APP + 137;
struct Command { ChannelCommand kind = ChannelCommand::disable; uint64_t id = 0; std::wstring text; };
struct Shared {
    std::mutex mutex;
    std::condition_variable changed;
    std::atomic<bool> stop{false}, sta_done{false};
    bool queued = false, replied = false;
    Command command;
    TargetReply reply;
    std::wstring selected;
    std::shared_ptr<ApplyConnectionLease> lease;
    std::weak_ptr<void> callbacks;
    HWND window = nullptr, field = nullptr;
    DWORD broker = 0, thread = 0;
    uint64_t nonce = 0;
    HMODULE module = nullptr;
    std::wstring class_name;
};
struct TargetSta {
    std::shared_ptr<Shared> shared;
    std::shared_ptr<void> callbacks = std::make_shared<int>(0);
    ComPtr<ITfThreadMgr> manager;
    TfClientId client = TF_CLIENTID_NULL;
    std::unique_ptr<DesktopTargetDispatcher> dispatcher;
    bool pending = false, polling = false;
    ChannelCommand operation = ChannelCommand::disable;
    explicit TargetSta(std::shared_ptr<Shared> value) : shared(std::move(value)) { shared->callbacks = callbacks; }
    ~TargetSta() {
        dispatcher.reset(); callbacks.reset();
        if (manager && client != TF_CLIENTID_NULL) manager->Deactivate();
    }
    void reply(DispatchStatus status, std::wstring text = {}) {
        std::lock_guard<std::mutex> lock(shared->mutex);
        shared->reply = {static_cast<uint32_t>(status), static_cast<uint32_t>(text.size()), dispatcher->capture_id()};
        shared->selected = std::move(text); shared->replied = true; pending = false;
        if (operation == ChannelCommand::apply) dispatcher->retire_completed_apply();
        shared->changed.notify_all();
    }
    void poll() {
        if (polling) return;
        polling = true;
        struct EndPoll { bool& value; ~EndPoll() { value = false; } } endPoll{polling};
        if (pending) {
            auto status = operation == ChannelCommand::capture ? dispatcher->capture_status() : dispatcher->apply_status();
            if (status != DispatchStatus::pending) {
                std::wstring selected;
                if (status == DispatchStatus::captured && !dispatcher->selected_text(dispatcher->capture_id(), selected)) status = DispatchStatus::rejected;
                reply(status, std::move(selected));
            }
            return;
        }
        Command command; std::shared_ptr<ApplyConnectionLease> lease;
        {
            std::lock_guard<std::mutex> lock(shared->mutex);
            if (!shared->queued) {
                if (operation == ChannelCommand::capture && dispatcher->capture_id()) dispatcher->capture_status();
                return;
            }
            command = std::move(shared->command); shared->command = Command{}; shared->queued = false; lease = shared->lease;
        }
        operation = command.kind;
        if (!lease || !lease->alive()) { reply(DispatchStatus::rejected); return; }
        if (operation == ChannelCommand::capture) {
            ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> context; ComPtr<ITfContextView> view; HWND actual = nullptr;
            // No document GetText before an authenticated capture request. The
            // broker checked this field identity in the greeting before sending.
            bool bound = SUCCEEDED(manager->GetFocus(&document)) && document && SUCCEEDED(document->GetTop(&context)) && context &&
                SUCCEEDED(context->GetActiveView(&view)) && view && SUCCEEDED(view->GetWnd(&actual)) && actual == shared->field;
            pending = bound && dispatcher->begin_capture(context.Get(), shared->field, true, lease);
            GRAMMAR_EDIT_DIAG("desktop_capture_admission bound=%d pending=%d\n",bound,pending);
        } else if (operation == ChannelCommand::apply) {
            const bool can=dispatcher->can_apply(command.id);
            const bool active=can&&dispatcher->activate(command.id,shared->field,shared->broker);
            pending=active&&dispatcher->begin_apply(command.id,std::move(command.text));
            GRAMMAR_EDIT_DIAG("standard_apply_admission can=%d active=%d pending=%d\n",can,active,pending);
        } else if (operation == ChannelCommand::cancel) {
            dispatcher->cancel_capture(command.id); reply(DispatchStatus::rejected); return;
        } else { dispatcher->cancel(); reply(DispatchStatus::rejected); return; }
        if (!pending) reply(DispatchStatus::rejected);
    }
};
void finish(HWND window, TargetSta* state) {
    auto shared = state->shared;
    if (state->polling) return;
    if (state->dispatcher && !state->dispatcher->close()) return;
    if (state->dispatcher) state->dispatcher.reset();
    state->callbacks.reset();
    if (state->manager && state->client != TF_CLIENTID_NULL) {
        state->manager->Deactivate(); state->client = TF_CLIENTID_NULL; state->manager.Reset();
    }
    // Observe release on a subsequent target-STA message, not just the worker:
    // a final COM Release must return before its DLL can be unloaded.
    if (!shared->callbacks.expired()) return;
    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
    KillTimer(window, 1); DestroyWindow(window);
    delete state; // All context/binding ownership released on the target STA.
    if (UnregisterClassW(shared->class_name.c_str(), shared->module)) shared->sta_done = true;
    shared->changed.notify_all();
}
LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM wp, LPARAM lp) {
    auto state = reinterpret_cast<TargetSta*>(GetWindowLongPtrW(window, GWLP_USERDATA));
    if (message == WM_NCCREATE) {
        state = static_cast<TargetSta*>(reinterpret_cast<CREATESTRUCTW*>(lp)->lpCreateParams);
        SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(state));
    }
    if (state && (message == wake_message || message == WM_TIMER || message == WM_CLOSE)) {
        try {
            if (message == WM_CLOSE) state->shared->stop = true;
            if (state->shared->stop.load()) { finish(window, state); return 0; }
            state->poll();
        } catch (...) { state->shared->stop = true; state->shared->changed.notify_all(); }
        return 0;
    }
    // External window messages never contain pointers, commands or authority.
    return DefWindowProcW(window, message, wp, lp);
}
std::wstring trusted_image(HMODULE module) {
    std::wstring path(32768, L'\0'); DWORD size = GetModuleFileNameW(module, path.data(), static_cast<DWORD>(path.size()));
    if (!size || size >= path.size()) return {};
    path.resize(size); auto slash = path.find_last_of(L"\\/");
    if (slash == std::wstring::npos) return {};
    return path.substr(0, slash + 1) + GRAMMAR_TRUSTED_APP_NAME;
}
bool trusted_digest(ApplyImageDigest& digest) {
    const std::string hex = GRAMMAR_TRUSTED_APP_SHA256;
    if (hex.size() != 64) return false;
    auto nibble = [](char c) { return c >= '0' && c <= '9' ? c-'0' : c >= 'a' && c <= 'f' ? c-'a'+10 : -1; };
    for (size_t i = 0; i < digest.size(); ++i) {
        int high = nibble(hex[i*2]), low = nibble(hex[i*2+1]);
        if (high < 0 || low < 0) return false;
        digest[i] = static_cast<unsigned char>((high << 4) | low);
    }
    return true;
}
void exchange(const std::shared_ptr<Shared>& shared) {
    ApplyImageDigest digest{};
    if (!trusted_digest(digest)) return;
    const auto connectionDeadline = GetTickCount64() + 180000; // Bounded review lifetime, not a per-stage reset.
    auto peer = verify_apply_peer(shared->broker, trusted_image(shared->module), digest, GetTickCount64() + 5000);
    if (!peer) return;
    OwnedApplyPipe pipe(target_pipe_name(shared->broker, GetCurrentProcessId(), shared->nonce));
    if (pipe.get() == INVALID_HANDLE_VALUE) return;
    auto connectDeadline = GetTickCount64() + 5000;
    bool connected = false;
    while (!shared->stop && peer->alive() && GetTickCount64() < connectDeadline) {
        if (ConnectNamedPipe(pipe.get(), nullptr) || GetLastError() == ERROR_PIPE_CONNECTED) { connected = true; break; }
        Sleep(5);
    }
    ApplyChannelAdmission admission(peer->process());
    if (!connected || !admission.peer_matches(pipe.get())) return;
    TargetHello hello;
    hello.window = reinterpret_cast<uintptr_t>(shared->field); hello.server_pid = GetCurrentProcessId();
    hello.window_thread = shared->thread; hello.worker_thread = GetCurrentThreadId(); hello.capability = admission.bootstrap_capability();
    bool sent = target_write(pipe.get(), &hello, sizeof(hello), GetTickCount64() + 1500);
    SecureZeroMemory(hello.capability.data(), hello.capability.size());
    if (!sent) return;
    while (!shared->stop && peer->alive() && GetTickCount64() < connectionDeadline) {
        DWORD bytes = 0;
        if (!PeekNamedPipe(pipe.get(), nullptr, 0, nullptr, &bytes, nullptr)) break;
        if (!bytes) { Sleep(10); continue; }
        ChannelRequest request;
        auto requestDeadline = (std::min)(connectionDeadline, GetTickCount64() + 1500);
        if (!receive_apply_request(pipe.get(), admission, request, requestDeadline)) break;
        if (request.header.command == ChannelCommand::disable) break;
        {
            std::lock_guard<std::mutex> lock(shared->mutex);
            if (shared->queued || !shared->selected.empty()) break;
            shared->lease = admission.lease(); shared->replied = false;
            shared->command.kind = request.header.command; shared->command.id = request.header.capture_id;
            for (size_t i = 0; i < request.body.size(); i += 2)
                shared->command.text.push_back(static_cast<wchar_t>(request.body[i] | request.body[i+1] << 8));
            shared->queued = true;
        }
        if (!PostMessageW(shared->window, wake_message, 0, 0)) break;
        TargetReply reply; std::wstring selected;
        auto operationDeadline = (std::min)(connectionDeadline, GetTickCount64() + 5000);
        {
            std::unique_lock<std::mutex> lock(shared->mutex);
            while (!shared->replied && !shared->stop && GetTickCount64() < operationDeadline) {
                if (!admission.peer_matches(pipe.get())) break;
                DWORD waiting = 0;
                if (!PeekNamedPipe(pipe.get(), nullptr, 0, nullptr, &waiting, nullptr)) break;
                if (waiting) {
                    // Cancellation cannot sit behind an outstanding STA reply.
                    // Revoke the lease immediately; a callback already committed
                    // remains applied/uncertain, never falsely "failed safely".
                    // Any pipelined frame terminates this single-operation
                    // connection. Never execute a second queued mutation.
                    admission.revoke(); shared->stop = true;
                    break;
                }
                shared->changed.wait_for(lock, std::chrono::milliseconds(10));
            }
            if (!shared->replied || shared->stop) break;
            reply = shared->reply; selected = std::move(shared->selected); shared->selected.clear();
        }
        auto replyDeadline = (std::min)(connectionDeadline, GetTickCount64()+1500);
        bool wrote = target_write(pipe.get(), &reply, sizeof(reply), replyDeadline) &&
            target_write(pipe.get(), selected.data(), static_cast<DWORD>(selected.size()*sizeof(wchar_t)), replyDeadline);
        if (!selected.empty()) SecureZeroMemory(selected.data(), selected.size()*sizeof(wchar_t));
        if (!wrote) break;
    }
    admission.revoke();
}
DWORD WINAPI worker(void* value) {
    HMODULE module = nullptr;
    {
        std::unique_ptr<std::shared_ptr<Shared>> argument(static_cast<std::shared_ptr<Shared>*>(value));
        auto shared = *argument; argument.reset(); module = shared->module;
        try { exchange(shared); } catch (...) {}
        {
            std::lock_guard<std::mutex> lock(shared->mutex);
            if (shared->lease) shared->lease->revoke();
            shared->stop = true;
        }
        PostMessageW(shared->window, wake_message, 0, 0);
        // A stuck target/TSF callback retains ONE explicitly owned worker/module;
        // callers time out cleanup and must not label it complete or start more.
        while (!shared->sta_done || !shared->callbacks.expired()) Sleep(10);
        occupied = false;
    }
    FreeLibraryAndExitThread(module, 0); // No C++ owners remain on this stack.
}
bool start(HWND field, DWORD broker, uint64_t nonce) {
    DWORD process = 0;
    if (!broker || !nonce || broker == GetCurrentProcessId() || GetWindowThreadProcessId(field, &process) != GetCurrentThreadId() ||
        process != GetCurrentProcessId()) return false;
    WCHAR name[64]{};
    if (!GetClassNameW(field, name, 64)) return false;
    const bool standard = _wcsicmp(name, L"Edit") == 0;
    // Without native auto-vertical scrolling, multiline Edit loses deleted
    // source from its Undo record. Never change an application's control style.
    // The no-auto-scroll form remains excluded after its preserved Undo failures.
    if ((!standard && _wcsicmp(name, L"RICHEDIT50W") != 0) ||
        (standard && (GetWindowLongPtrW(field, GWL_STYLE) & ES_MULTILINE) &&
            !(GetWindowLongPtrW(field, GWL_STYLE) & ES_AUTOVSCROLL)) || occupied.exchange(true)) return false;
    struct ModuleOwner {
        HMODULE module = nullptr; bool transferred = false;
        ~ModuleOwner() { if (!transferred) { if (module) FreeLibrary(module); occupied = false; } }
    } owner;
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, reinterpret_cast<LPCWSTR>(&start), &module)) return false;
    owner.module = module;
    auto shared = std::make_shared<Shared>(); shared->module = module; shared->field = field;
    shared->broker = broker; shared->nonce = nonce; shared->thread = GetCurrentThreadId();
    shared->class_name = L"Grammar.TargetApply." + std::to_wstring(nonce);
    auto state = std::make_unique<TargetSta>(shared);
    bool activated = SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&state->manager))) &&
        SUCCEEDED(state->manager->Activate(&state->client));
    if (!activated) return false;
    state->dispatcher = std::make_unique<DesktopTargetDispatcher>(state->client, standard, state->callbacks);
    struct WindowOwner {
        std::shared_ptr<Shared> shared; bool registered = false, transferred = false;
        ~WindowOwner() {
            if (transferred) return;
            if (shared->window) { SetWindowLongPtrW(shared->window,GWLP_USERDATA,0); KillTimer(shared->window,1); DestroyWindow(shared->window); }
            if (registered) UnregisterClassW(shared->class_name.c_str(),shared->module);
        }
    } windowOwner{shared};
    WNDCLASSW wc{}; wc.hInstance = module; wc.lpfnWndProc = window_proc; wc.lpszClassName = shared->class_name.c_str();
    if (!RegisterClassW(&wc)) return false;
    windowOwner.registered = true;
    HWND window = CreateWindowExW(0, wc.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE, nullptr, module, state.get());
    shared->window = window;
    if (!window || !SetTimer(window, 1, 10, nullptr)) return false;
    auto argument = std::make_unique<std::shared_ptr<Shared>>(shared);
    HANDLE thread = CreateThread(nullptr, 0, worker, argument.get(), 0, nullptr);
    if (!thread) return false;
    argument.release(); windowOwner.transferred = true; owner.transferred = true;
    CloseHandle(thread); state.release(); return true;
}
}
extern "C" __declspec(dllexport) LRESULT CALLBACK GrammarApplyHook(int code, WPARAM wp, LPARAM lp) {
    if (code == HC_ACTION) {
        auto message = reinterpret_cast<const CWPSTRUCT*>(lp);
        if (message && message->message == RegisterWindowMessageW(target_bootstrap_message)) {
            try { start(message->hwnd, static_cast<DWORD>(message->wParam), static_cast<uint64_t>(message->lParam)); }
            catch (...) { /* no exception crosses the target window procedure */ }
        }
    }
    return CallNextHookEx(nullptr, code, wp, lp);
}
