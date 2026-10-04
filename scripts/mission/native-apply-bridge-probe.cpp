#include "../../src-tauri/native/tsf_dispatcher.hpp"
#include <cstdio>
using namespace grammar;
struct Reply { uint32_t status; uint32_t units; uint64_t capture_id; };
static void pump(DWORD ms) {
    auto end = GetTickCount64() + ms;
    do { MSG message;
        while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
        Sleep(1);
    } while (GetTickCount64() < end);
}
static bool write_all(HANDLE pipe, const void* value, DWORD size) {
    auto bytes = static_cast<const unsigned char*>(value); auto end = GetTickCount64() + 1500;
    while (size && GetTickCount64() < end) {
        DWORD written = 0;
        if (!WriteFile(pipe, bytes, size > 1024 ? 1024 : size, &written, nullptr)) return false;
        bytes += written; size -= written; if (!written) Sleep(1);
    }
    return !size;
}
static int broker(wchar_t** argv) {
    ChannelHandle bootstrap(reinterpret_cast<HANDLE>(_wcstoui64(argv[2], nullptr, 10)));
    ChannelHeader request; DWORD received = 0;
    if (!ReadFile(bootstrap.get(), request.capability.data(), 32, &received, nullptr) || received != 32) return 20;
    ChannelHandle pipe(CreateFileW(argv[3], GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
        SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, nullptr));
    ULONG server = 0; DWORD mode = PIPE_NOWAIT;
    if (pipe.get() == INVALID_HANDLE_VALUE || !GetNamedPipeServerProcessId(pipe.get(), &server) ||
        server != wcstoul(argv[4], nullptr, 10) || !SetNamedPipeHandleState(pipe.get(), &mode, nullptr, nullptr)) return 21;
    int scenario = _wtoi(argv[5]);
    if (!write_all(pipe.get(), &request, sizeof(request))) return 22;
    Reply response{};
    if (!channel_read_exact(pipe.get(), &response, sizeof(response), GetTickCount64() + 3000)) return 23;
    if (scenario >= 9) return response.status == static_cast<uint32_t>(DispatchStatus::rejected) ? 0 : 24;
    if (response.status != static_cast<uint32_t>(DispatchStatus::captured) || response.units != 5 || response.capture_id != 1) return 25;
    WCHAR selected[5]{};
    if (!channel_read_exact(pipe.get(), selected, sizeof(selected), GetTickCount64() + 1500) || std::wstring(selected, 5) != L"world") return 26;
    request.sequence = 2; request.capture_id = scenario == 1 ? 999 : response.capture_id;
    request.command = ChannelCommand::apply; request.payload_bytes = 14;
    if (!write_all(pipe.get(), &request, sizeof(request)) || !write_all(pipe.get(), L"Grammar", 14)) return 27;
    if (scenario == 6) {
        request.sequence = 3; request.command = ChannelCommand::cancel; request.payload_bytes = 0;
        if (!write_all(pipe.get(), &request, sizeof(request))) return 32;
    }
    if (!channel_read_exact(pipe.get(), &response, sizeof(response), GetTickCount64() + 3000)) return 28;
    if (scenario == 5) return response.status == 99 ? 0 : 29;
    const bool normal = scenario == 0 || scenario == 8;
    if (response.status != static_cast<uint32_t>(normal ? DispatchStatus::applied : DispatchStatus::rejected)) return 30;
    if (scenario == 8) {
        request.sequence = 3;
        if (!write_all(pipe.get(), &request, sizeof(request)) || !write_all(pipe.get(), L"Grammar", 14) ||
            !channel_read_exact(pipe.get(), &response, sizeof(response), GetTickCount64() + 1500) ||
            response.status != static_cast<uint32_t>(DispatchStatus::rejected)) return 31;
    }
    SecureZeroMemory(request.capability.data(), request.capability.size());
    return 0;
}
int wmain(int argc, wchar_t** argv) {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount") return 2;
    if (argc == 6 && std::wstring(argv[1]) == L"--broker") return broker(argv);
    if (FAILED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) return 3;
    int failures = 0;
    {
        ComPtr<ITfThreadMgr> manager; TfClientId client = 0;
        if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client))) return 4;
        LoadLibraryW(L"Msftedit.dll");
        HWND top = CreateWindowExW(0, L"STATIC", L"Grammar owned cross-process Apply", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            30, 30, 600, 250, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
        HWND edit = CreateWindowExW(0, L"RICHEDIT50W", L"hello world", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
            10, 10, 550, 150, top, nullptr, GetModuleHandleW(nullptr), nullptr);
        if (!top || !edit) return 5;
        // First call honors the hidden launcher STARTUPINFO; second shows our
        // synthetic guest window only (never a normal user desktop).
        ShowWindow(top, SW_SHOW); ShowWindow(top, SW_SHOW);
        auto inputLibrary = LoadLibraryExW(L"msctf.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
        using SetScope = HRESULT(WINAPI*)(HWND, InputScope);
        auto setScope = inputLibrary ? reinterpret_cast<SetScope>(GetProcAddress(inputLibrary, "SetInputScope")) : nullptr;
        if (!setScope) return 12;
        SendMessageW(edit, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
        SetForegroundWindow(top); SetFocus(edit); pump(500);
        ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> context;
        if (FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context) return 6;
        WCHAR executable[32768]{}; GetModuleFileNameW(nullptr, executable, 32768);
        // Unknown default scope remains a denial case. Other scenarios model
        // a host explicitly declaring plain text, never a runtime override.
        for (int scenario = 11; scenario >= 0; --scenario) {
            if (scenario != 11 && FAILED(setScope(edit, IS_TEXT))) return 13;
            SendMessageW(edit, EM_SETPASSWORDCHAR, 0, 0);
            SetWindowTextW(edit, L"hello world"); SendMessageW(edit, EM_SETSEL, 6, 11); pump(30);
            if (scenario == 10) SendMessageW(edit, EM_SETPASSWORDCHAR, L'*', 0);
            OwnedApplyPipe pipe; ConnectNamedPipe(pipe.get(), nullptr);
            SECURITY_ATTRIBUTES inherit{sizeof(inherit), nullptr, TRUE}; HANDLE input = nullptr, output = nullptr;
            if (!CreatePipe(&input, &output, &inherit, 64)) return 7;
            ChannelHandle reader(input), writer(output); SetHandleInformation(output, HANDLE_FLAG_INHERIT, 0);
            SIZE_T size = 0; InitializeProcThreadAttributeList(nullptr, 1, 0, &size);
            std::vector<unsigned char> attributes(size); auto list = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(attributes.data());
            if (!InitializeProcThreadAttributeList(list, 1, 0, &size) ||
                !UpdateProcThreadAttribute(list, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &input, sizeof(input), nullptr, nullptr)) return 8;
            STARTUPINFOEXW startup{}; startup.StartupInfo.cb = sizeof(startup); startup.lpAttributeList = list; PROCESS_INFORMATION info{};
            auto command = L"\"" + std::wstring(executable) + L"\" --broker " + std::to_wstring(reinterpret_cast<uintptr_t>(input)) +
                L" " + pipe.name + L" " + std::to_wstring(GetCurrentProcessId()) + L" " + std::to_wstring(scenario);
            BOOL created = CreateProcessW(executable, command.data(), nullptr, nullptr, TRUE,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW, nullptr, nullptr, &startup.StartupInfo, &info);
            DeleteProcThreadAttributeList(list); if (!created) return 9;
            ChannelHandle process(info.hProcess), thread(info.hThread); ApplyChannelAdmission admission(process.get());
            DWORD written = 0;
            if (!admission.alive() || !WriteFile(output, admission.bootstrap_capability().data(), 32, &written, nullptr) || written != 32) {
                TerminateProcess(process.get(), 99); WaitForSingleObject(process.get(), 3000); return 10;
            }
            ResumeThread(thread.get()); auto connectionEnd = GetTickCount64() + 1500;
            while (!ConnectNamedPipe(pipe.get(), nullptr) && GetLastError() != ERROR_PIPE_CONNECTED && GetTickCount64() < connectionEnd) Sleep(1);
            TsfTargetDispatcher dispatcher(client); ChannelRequest request;
            bool pass = receive_apply_request(pipe.get(), admission, request, GetTickCount64() + 1500) &&
                request.header.command == ChannelCommand::capture && dispatcher.begin_capture(context.Get(), edit, true, admission.lease());
            if (scenario == 9) dispatcher.cancel();
            auto captureEnd = GetTickCount64() + 1500;
            while (dispatcher.capture_status() == DispatchStatus::pending && GetTickCount64() < captureEnd) pump(5);
            auto status = dispatcher.capture_status(); std::wstring selected;
            printf("capture case=%d admitted=%d status=%u called=%d eligibility=%u reads=%u lease=%d\n", scenario, pass,
                static_cast<uint32_t>(status), dispatcher.capture_called(), dispatcher.eligibility_stage(), dispatcher.capture_reads(),
                admission.lease() && admission.lease()->alive());
            bool denied = scenario >= 9;
            pass = pass && status == (denied ? DispatchStatus::rejected : DispatchStatus::captured);
            if (!denied) pass = pass && dispatcher.selected_text(1, selected) && selected == L"world";
            Reply response{static_cast<uint32_t>(status), static_cast<uint32_t>(selected.size()), dispatcher.capture_id()};
            pass = write_all(pipe.get(), &response, sizeof(response)) && pass;
            if (!selected.empty()) pass = write_all(pipe.get(), selected.data(), static_cast<DWORD>(selected.size() * 2)) && pass;
            if (!denied) {
                bool received = receive_apply_request(pipe.get(), admission, request, GetTickCount64() + 1500);
                pass = pass && received && request.header.command == ChannelCommand::apply;
                if (scenario == 2) SetWindowTextW(edit, L"application edit");
                if (scenario == 3) { SetWindowTextW(edit, L"temporary"); SetWindowTextW(edit, L"hello world"); SendMessageW(edit, EM_SETSEL, 6, 11); }
                if (scenario == 4) { SendMessageW(edit, EM_SETSEL, 0, 5); SendMessageW(edit, EM_SETSEL, 6, 11); }
                std::wstring replacement;
                for (size_t i = 0; i + 1 < request.body.size(); i += 2) replacement.push_back(static_cast<wchar_t>(request.body[i] | request.body[i + 1] << 8));
                bool admitted = received && dispatcher.begin_apply(request.header.capture_id, std::move(replacement));
                if (scenario == 5) {
                    response = Reply{99, 0, 1}; pass = write_all(pipe.get(), &response, sizeof(response)) && pass;
                    pass = WaitForSingleObject(process.get(), 3000) == WAIT_OBJECT_0 && pass;
                }
                if (scenario == 6) {
                    bool cancelled = receive_apply_request(pipe.get(), admission, request, GetTickCount64() + 1500) &&
                        request.header.command == ChannelCommand::cancel && dispatcher.cancel_capture(request.header.capture_id);
                    pass = cancelled && pass;
                }
                if (scenario == 7) SendMessageW(edit, EM_SETPASSWORDCHAR, L'*', 0);
                auto applyEnd = GetTickCount64() + 1500;
                while (dispatcher.apply_status() == DispatchStatus::pending && GetTickCount64() < applyEnd) pump(5);
                status = dispatcher.apply_status();
                const bool normal = scenario == 0 || scenario == 8;
                pass = pass && status == (normal ? DispatchStatus::applied : DispatchStatus::rejected) && (!normal || admitted);
                response = Reply{static_cast<uint32_t>(status), 0, 1};
                if (scenario != 5) pass = write_all(pipe.get(), &response, sizeof(response)) && pass;
                if (scenario == 8) {
                    pass = receive_apply_request(pipe.get(), admission, request, GetTickCount64() + 1500) && pass;
                    pass = !dispatcher.begin_apply(request.header.capture_id, L"Grammar") && pass;
                    response.status = static_cast<uint32_t>(DispatchStatus::rejected);
                    pass = write_all(pipe.get(), &response, sizeof(response)) && pass;
                }
            }
            WCHAR actual[128]{}; GetWindowTextW(edit, actual, 128);
            const auto expected = scenario == 0 || scenario == 8 ? L"hello Grammar" : scenario == 2 ? L"application edit" : L"hello world";
            pass = pass && std::wstring(actual) == expected && dispatcher.mutation_calls() == (scenario == 0 || scenario == 8 ? 1u : 0u);
            bool exited = WaitForSingleObject(process.get(), 3000) == WAIT_OBJECT_0; DWORD exitCode = STILL_ACTIVE; GetExitCodeProcess(process.get(), &exitCode);
            pass = pass && exited && exitCode == 0;
            printf("case=%d pass=%d status=%u mutations=%u reread=%d broker_pid=%lu exited=%d exit=%lu\n", scenario, pass,
                static_cast<uint32_t>(status), dispatcher.mutation_calls(), std::wstring(actual) == expected, info.dwProcessId, exited, exitCode);
            admission.revoke(); dispatcher.cancel(); DisconnectNamedPipe(pipe.get()); pump(10);
            if (!pass) ++failures;
            if (!exited) return 11;
        }
        DestroyWindow(top); manager->Deactivate(); FreeLibrary(inputLibrary);
    }
    CoUninitialize(); printf("failures=%d\n", failures); return failures ? 1 : 0;
}
