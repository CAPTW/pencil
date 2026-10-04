// Real local pipe/process qualification in an owned Medium Windows Sandbox.
#include "../../src-tauri/native/apply_channel.hpp"
#include <cstdio>
#include <cstdlib>
using namespace grammar;

static bool write_bytes(HANDLE pipe, const void* data, DWORD size, ULONGLONG deadline) {
    auto cursor = static_cast<const unsigned char*>(data);
    while (size && GetTickCount64() < deadline) {
        DWORD written = 0;
        if (!WriteFile(pipe, cursor, size > 1024 ? 1024 : size, &written, nullptr)) return false;
        cursor += written; size -= written;
        if (size && !written) Sleep(1);
    }
    return !size;
}
static int child(int argc, wchar_t** argv) {
    if (argc != 7) return 30;
    ChannelHandle bootstrap(reinterpret_cast<HANDLE>(_wcstoui64(argv[2], nullptr, 10)));
    ChannelCapability capability{}; DWORD read = 0;
    if (!ReadFile(bootstrap.get(), capability.data(), static_cast<DWORD>(capability.size()), &read, nullptr) || read != capability.size()) return 31;
    ChannelHandle pipe(CreateFileW(argv[3], GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
        SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, nullptr));
    if (pipe.get() == INVALID_HANDLE_VALUE) return 32;
    ULONG server = 0;
    if (!GetNamedPipeServerProcessId(pipe.get(), &server) || server != wcstoul(argv[4], nullptr, 10)) return 41;
    DWORD mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
    if (!SetNamedPipeHandleState(pipe.get(), &mode, nullptr, nullptr)) return 33;
    int scenario = _wtoi(argv[5]);
    ChannelHeader header; header.capability = capability;
    std::vector<unsigned char> body;
    if (scenario == 1 || scenario == 12 || scenario >= 22) {
        header.command = ChannelCommand::apply;
        header.payload_bytes = scenario == 22 ? channel_max_payload : 8;
        body.resize(header.payload_bytes);
        for (size_t i = 0; i < body.size(); i += 2) body[i] = 'x';
        if (scenario == 23) body[0] = 0; // Embedded NUL.
        if (scenario == 24) { body[0] = 0; body[1] = 0xd8; } // Unpaired high surrogate.
        if (scenario == 25) { body[0] = 0; body[1] = 0xdc; } // Unpaired low surrogate.
        if (scenario == 26) { body[0] = 0x3d; body[1] = 0xd8; body[2] = 0; body[3] = 0xde; }
    }
    if (scenario == 2) header.capability[0] ^= 1;
    if (scenario == 4) header.version = 3;
    if (scenario == 5) { header.command = ChannelCommand::apply; header.payload_bytes = channel_max_payload + 2; }
    if (scenario == 6) { header.command = ChannelCommand::apply; header.payload_bytes = 3; }
    if (scenario == 8) header.sequence = 2;
    if (scenario == 10) header.command = ChannelCommand::disable;
    if (scenario == 14) header.command = static_cast<ChannelCommand>(99);
    if (scenario == 15) header.command = ChannelCommand::apply;
    if (scenario == 16) header.payload_bytes = 2;
    if (scenario == 17) header.reserved = 1;
    if (scenario == 18) header.sequence = (std::numeric_limits<uint64_t>::max)();
    if (header.command == ChannelCommand::apply) header.capture_id = 1; // Admission-only fixture, no document authority.
    if (scenario >= 28) {
        body.clear(); header.payload_bytes = 0;
        header.command = scenario == 28 ? ChannelCommand::capture : scenario == 32 ? ChannelCommand::disable : ChannelCommand::cancel;
        header.capture_id = scenario == 30 ? 0 : 1;
        if (scenario == 29) { header.command = ChannelCommand::apply; header.capture_id = 0; header.payload_bytes = 2; body = {'x', 0}; }
    }
    auto deadline = GetTickCount64() + 2000;
    if (scenario == 27) {
        if (!write_bytes(pipe.get(), &header, 8, deadline)) return 34;
        Sleep(100);
        if (!write_bytes(pipe.get(), reinterpret_cast<const unsigned char*>(&header) + 8, sizeof(header) - 8, deadline)) return 34;
    } else if (!write_bytes(pipe.get(), &header, scenario == 11 ? 8 : sizeof(header), deadline)) return 34;
    if (!body.empty() && !write_bytes(pipe.get(), body.data(), scenario == 12 || scenario == 27 ? 2 : static_cast<DWORD>(body.size()), deadline)) return 35;
    if (scenario == 7 || scenario == 10) {
        if (scenario == 10) { header.sequence = 2; header.command = ChannelCommand::capture; }
        if (!write_bytes(pipe.get(), &header, sizeof(header), deadline)) return 36;
    }
    SecureZeroMemory(capability.data(), capability.size());
    SecureZeroMemory(header.capability.data(), header.capability.size());
    if (scenario == 13) return 0;
    unsigned char ack = 0;
    channel_read_exact(pipe.get(), &ack, 1, deadline);
    return 0; // Rejected requests are closed without a payload response.
}
int wmain(int argc, wchar_t** argv) {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount") return 2;
    if (argc > 1 && std::wstring(argv[1]) == L"--child") return child(argc, argv);
    WCHAR executable[32768]{};
    if (!GetModuleFileNameW(nullptr, executable, 32768)) return 3;
    int failures = 0;
    for (int scenario = 0; scenario < 33; ++scenario) {
        OwnedApplyPipe pipe;
        if (pipe.get() == INVALID_HANDLE_VALUE) { printf("pipe_creation_failed=%lu\n", GetLastError()); return 4; }
        ConnectNamedPipe(pipe.get(), nullptr); // Nonblocking: begin listening.
        SECURITY_ATTRIBUTES inherit{sizeof(inherit), nullptr, TRUE}; HANDLE input = nullptr, output = nullptr;
        if (!CreatePipe(&input, &output, &inherit, 64)) return 5;
        ChannelHandle reader(input), writer(output);
        if (!SetHandleInformation(writer.get(), HANDLE_FLAG_INHERIT, 0)) return 6;
        SIZE_T attributeBytes = 0;
        InitializeProcThreadAttributeList(nullptr, 1, 0, &attributeBytes);
        std::vector<unsigned char> attributeBuffer(attributeBytes);
        auto attributes = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(attributeBuffer.data());
        if (!InitializeProcThreadAttributeList(attributes, 1, 0, &attributeBytes) ||
            !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &input, sizeof(input), nullptr, nullptr)) return 7;
        STARTUPINFOEXW startup{}; startup.StartupInfo.cb = sizeof(startup); startup.lpAttributeList = attributes;
        PROCESS_INFORMATION info{};
        auto command = L"\"" + std::wstring(executable) + L"\" --child " +
            std::to_wstring(reinterpret_cast<uintptr_t>(input)) + L" " + pipe.name + L" " +
            std::to_wstring(GetCurrentProcessId() + (scenario == 19 ? 1 : 0)) + L" " + std::to_wstring(scenario) + L" fixture";
        BOOL created = CreateProcessW(executable, command.data(), nullptr, nullptr, TRUE,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW, nullptr, nullptr, &startup.StartupInfo, &info);
        DeleteProcThreadAttributeList(attributes);
        if (!created) return 8;
        ChannelHandle process(info.hProcess), thread(info.hThread);
        ApplyChannelAdmission admission(scenario == 3 ? GetCurrentProcess() : process.get());
        DWORD sent = 0;
        bool ready = admission.alive() && WriteFile(writer.get(), admission.bootstrap_capability().data(), 32, &sent, nullptr) && sent == 32;
        ResumeThread(thread.get());
        if (!ready) {
            // Exact just-created fixture process; cleanup is not a normal exit.
            TerminateProcess(process.get(), 99); WaitForSingleObject(process.get(), 3000);
            puts("admission_setup_failed_forced_fixture_cleanup"); return 9;
        }
        auto deadline = GetTickCount64() + 1500;
        bool connected = false;
        do {
            if (ConnectNamedPipe(pipe.get(), nullptr) || GetLastError() == ERROR_PIPE_CONNECTED) { connected = true; break; }
            Sleep(1);
        } while (GetTickCount64() < deadline);
        if (scenario == 9) admission.revoke();
        if (scenario == 13) WaitForSingleObject(process.get(), 3000);
        if (scenario == 21) { DWORD blocking = PIPE_WAIT; SetNamedPipeHandleState(pipe.get(), &blocking, nullptr, nullptr); }
        ChannelRequest request;
        request.body.assign(16, 0x55); // A rejected/reused request must clear old content.
        auto started = GetTickCount64();
        // Invoke fail-closed receive even if the child exited before connection
        // observation; short-circuiting would skip old-buffer clearing/revocation.
        bool accepted = receive_apply_request(pipe.get(), admission, request,
            started + ((scenario == 11 || scenario == 12 || scenario == 27) ? 150 : 1500));
        auto elapsed = GetTickCount64() - started;
        bool expected = scenario == 0 || scenario == 1 || scenario == 7 || scenario == 10 || scenario == 20 || scenario == 22 || scenario == 26 || scenario == 31;
        bool pass = accepted == expected && (!accepted || connected);
        if (scenario == 7 || scenario == 10) {
            ChannelRequest repeated;
            pass = pass && !receive_apply_request(pipe.get(), admission, repeated, GetTickCount64() + 500) && !admission.alive();
        }
        if (!accepted) pass = pass && request.body.empty() && !admission.alive();
        if (scenario == 1 || scenario == 22) pass = pass && request.body.size() == (scenario == 22 ? channel_max_payload : 8);
        if (scenario == 20) {
            ChannelHandle duplicate(CreateNamedPipeW(pipe.name.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS, 1, 4096, 4096, 0, nullptr));
            pass = pass && duplicate.get() == INVALID_HANDLE_VALUE;
        }
        for (auto byte : request.header.capability) if (byte) pass = false;
        if (scenario == 11 || scenario == 12) pass = pass && elapsed >= 150 && elapsed < 1000;
        // The delayed header consumed100ms of the same150ms deadline. A fresh
        //150ms payload budget would exceed225ms in this unloaded fixture.
        if (scenario == 27) pass = pass && elapsed >= 150 && elapsed < 225;
        DisconnectNamedPipe(pipe.get());
        bool exited = WaitForSingleObject(process.get(), 3000) == WAIT_OBJECT_0;
        DWORD exitCode = STILL_ACTIVE; GetExitCodeProcess(process.get(), &exitCode);
        pass = pass && exited && exitCode == (scenario == 19 ? 41u : 0u);
        printf("case=%d pass=%d accepted=%d bytes=%zu elapsed_ms=%llu child_exited=%d child_exit=%lu\n",
            scenario, pass, accepted, request.body.size(), elapsed, exited, exitCode);
        if (!pass) ++failures;
        if (!exited) return 10; // Do not spawn more children after incomplete cleanup.
    }
    printf("failures=%d\n", failures); return failures ? 1 : 0;
}
