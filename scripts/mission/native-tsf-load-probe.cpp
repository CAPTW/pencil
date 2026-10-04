// Diagnostic service and loader only. No text, keys, capture or mutation.
#include <windows.h>
#include <msctf.h>
#include <wrl/client.h>
#include <new>
#include <string>
#include <cstdio>
using Microsoft::WRL::ComPtr;
static constexpr GUID service_id = {0x7dc8373f,0x68ae,0x477c,{0xb6,0xb8,0xa7,0x39,0x87,0x10,0xaa,0x35}};
static constexpr GUID profile_id = {0x31e7bb4a,0xb013,0x43bf,{0xb0,0xad,0x72,0x16,0xf5,0x9b,0x88,0x02}};
static constexpr WCHAR bootstrap_message[] = L"Grammar.OwnedTsfLoadProbe.7dc8373f";
static constexpr WCHAR result_property[] = L"Grammar.OwnedTsfLoadProbe.Result";
#ifdef GRAMMAR_TSF_PROBE_DLL
static LONG objects = 0, locks = 0, activations = 0, active = 0;
class ProbeService final : public ITfTextInputProcessorEx {
    LONG refs_ = 1;
    ComPtr<ITfThreadMgr> manager_;
public:
    ProbeService() { InterlockedIncrement(&objects); }
    ~ProbeService() { Deactivate(); InterlockedDecrement(&objects); }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (id != IID_IUnknown && id != IID_ITfTextInputProcessor && id != IID_ITfTextInputProcessorEx) return E_NOINTERFACE;
        *out = static_cast<ITfTextInputProcessorEx*>(this); AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE Activate(ITfThreadMgr* manager, TfClientId client) override { return ActivateEx(manager, client, 0); }
    HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr* manager, TfClientId client, DWORD) override {
        if (!manager || client == TF_CLIENTID_NULL || manager_) return E_INVALIDARG;
        manager_ = manager; InterlockedIncrement(&activations); InterlockedIncrement(&active); return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Deactivate() override {
        if (manager_) { manager_.Reset(); InterlockedDecrement(&active); }
        return S_OK;
    }
};
class Factory final : public IClassFactory {
    LONG refs_ = 1;
public:
    Factory() { InterlockedIncrement(&objects); }
    ~Factory() { InterlockedDecrement(&objects); }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (id != IID_IUnknown && id != IID_IClassFactory) return E_NOINTERFACE;
        *out = static_cast<IClassFactory*>(this); AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        auto object = new(std::nothrow) ProbeService;
        if (!object) return E_OUTOFMEMORY;
        HRESULT hr = object->QueryInterface(id, out); object->Release(); return hr;
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
        if (lock) InterlockedIncrement(&locks); else InterlockedDecrement(&locks);
        return S_OK;
    }
};
#pragma comment(linker, "/export:DllGetClassObject,PRIVATE")
#pragma comment(linker, "/export:DllCanUnloadNow,PRIVATE")
STDAPI DllGetClassObject(REFCLSID clsid, REFIID id, void** out) {
    if (!out) return E_POINTER;
    *out = nullptr;
    if (clsid != service_id) return CLASS_E_CLASSNOTAVAILABLE;
    auto factory = new(std::nothrow) Factory;
    if (!factory) return E_OUTOFMEMORY;
    HRESULT hr = factory->QueryInterface(id, out); factory->Release(); return hr;
}
STDAPI DllCanUnloadNow() { return objects == 0 && locks == 0 ? S_OK : S_FALSE; }
extern "C" __declspec(dllexport) LONG WINAPI GrammarProbeActivations() { return InterlockedCompareExchange(&activations, 0, 0); }
extern "C" __declspec(dllexport) LONG WINAPI GrammarProbeActive() { return InterlockedCompareExchange(&active, 0, 0); }
// Diagnostic ONLY. A nonzero exact owned target thread is supplied by the
// launcher. No keyboard hook, global hook, key/text inspection or persistence.
extern "C" __declspec(dllexport) LRESULT CALLBACK GrammarOwnedBootstrapHook(int code, WPARAM wp, LPARAM lp) {
    if (code != HC_ACTION) return CallNextHookEx(nullptr, code, wp, lp);
    auto message = reinterpret_cast<const CWPSTRUCT*>(lp);
    if (!message || message->message != RegisterWindowMessageW(bootstrap_message)) return CallNextHookEx(nullptr, code, wp, lp);
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    DWORD pid = 0;
    if (std::wstring(user) != L"WDAGUtilityAccount" || GetWindowThreadProcessId(message->hwnd, &pid) != GetCurrentThreadId() || pid != GetCurrentProcessId())
        return CallNextHookEx(nullptr, code, wp, lp);
    static thread_local bool busy = false;
    static thread_local ITfThreadMgr* manager = nullptr;
    static thread_local ITfInputProcessorProfileMgr* profiles = nullptr;
    static thread_local LANGID language = 0;
    if (busy) return CallNextHookEx(nullptr, code, wp, lp);
    busy = true;
    if (message->wParam == 1 && !manager) {
        ComPtr<ITfThreadMgr> nextManager; ComPtr<ITfInputProcessorProfileMgr> nextProfiles; TfClientId client = TF_CLIENTID_NULL;
        if (SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&nextManager))) &&
            SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&nextProfiles))) &&
            SUCCEEDED(nextManager->Activate(&client))) {
            language = LOWORD(reinterpret_cast<ULONG_PTR>(GetKeyboardLayout(0)));
            constexpr WCHAR description[] = L"Grammar owned thread bootstrap probe";
            HRESULT registered = nextProfiles->RegisterProfile(service_id, language, profile_id, description, ARRAYSIZE(description)-1,
                L"", 0, 0, nullptr, 0, TRUE, TF_RP_LOCALPROCESS);
            HRESULT activated = SUCCEEDED(registered) ? nextProfiles->ActivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, language,
                service_id, profile_id, nullptr, TF_IPPMF_FORPROCESS) : registered;
            if (SUCCEEDED(activated)) { manager = nextManager.Detach(); profiles = nextProfiles.Detach(); }
            else nextManager->Deactivate();
        }
    } else if (message->wParam == 2 && manager) {
        profiles->DeactivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, language, service_id, profile_id, nullptr, TF_IPPMF_FORPROCESS);
        profiles->ReleaseInputProcessor(service_id, 0); profiles->Release(); profiles = nullptr;
        manager->Deactivate(); manager->Release(); manager = nullptr;
    }
    SetPropW(message->hwnd, result_property, reinterpret_cast<HANDLE>(static_cast<INT_PTR>(active) + 1));
    busy = false;
    return CallNextHookEx(nullptr, code, wp, lp);
}
#else
#include "../../src-tauri/native/apply_channel.hpp"
#include <richedit.h>
static void pump(DWORD milliseconds) {
    ULONGLONG end = GetTickCount64() + milliseconds;
    do {
        MSG message;
        while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
        Sleep(1);
    } while (GetTickCount64() < end);
}
static LONG exists(HKEY root, const std::wstring& path) {
    HKEY key = nullptr; LONG result = RegOpenKeyExW(root, path.c_str(), 0, KEY_READ, &key);
    if (key) RegCloseKey(key); return result;
}
static bool value(HKEY key, const WCHAR* name, const std::wstring& text) {
    return RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(text.c_str()),
        static_cast<DWORD>((text.size()+1)*sizeof(wchar_t))) == ERROR_SUCCESS;
}
static LRESULT CALLBACK target_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    if (msg == WM_DESTROY) { PostQuitMessage(0); return 0; }
    return DefWindowProcW(hwnd, msg, wp, lp);
}
static int target() {
    if (FAILED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) return 20;
    ComPtr<ITfThreadMgr> manager; TfClientId client = TF_CLIENTID_NULL;
    if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client))) return 21;
    LoadLibraryExW(L"Msftedit.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    WNDCLASSW wc{}; wc.lpfnWndProc = target_proc; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"GrammarTsfLoadTarget";
    if (!RegisterClassW(&wc)) return 22;
    HWND top = CreateWindowW(wc.lpszClassName, L"Grammar owned bootstrap target", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        40, 40, 500, 200, nullptr, nullptr, wc.hInstance, nullptr);
    HWND edit = CreateWindowW(L"RICHEDIT50W", L"synthetic target unchanged", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
        10, 10, 450, 120, top, nullptr, wc.hInstance, nullptr);
    if (!top || !edit) return 23;
    ShowWindow(top, SW_SHOW); ShowWindow(top, SW_SHOW);
    SendMessageW(edit, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF); SetForegroundWindow(top); SetFocus(edit);
    SetPropW(top, L"GrammarOwnedTargetReady", reinterpret_cast<HANDLE>(1));
    MSG message;
    while (GetMessageW(&message, nullptr, 0, 0) > 0) { TranslateMessage(&message); DispatchMessageW(&message); }
    manager->Deactivate(); manager.Reset(); CoUninitialize(); return 0;
}
static bool target_bootstrap(HMODULE module) {
    WCHAR executable[32768]{}; GetModuleFileNameW(nullptr, executable, 32768);
    auto command = L"\"" + std::wstring(executable) + L"\" --target";
    STARTUPINFOW startup{}; startup.cb = sizeof(startup); startup.dwFlags = STARTF_USESHOWWINDOW; startup.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION info{};
    if (!CreateProcessW(executable, command.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW, nullptr, nullptr, &startup, &info)) return false;
    grammar::ChannelHandle process(info.hProcess), thread(info.hThread);
    struct Search { DWORD pid; HWND window; } search{info.dwProcessId, nullptr};
    auto end = GetTickCount64() + 3000;
    do {
        EnumWindows([](HWND hwnd, LPARAM value) -> BOOL {
            auto search = reinterpret_cast<Search*>(value); DWORD pid = 0; GetWindowThreadProcessId(hwnd, &pid);
            WCHAR name[64]{}; GetClassNameW(hwnd, name, 64);
            if (pid == search->pid && std::wstring(name) == L"GrammarTsfLoadTarget") { search->window = hwnd; return FALSE; }
            return TRUE;
        }, reinterpret_cast<LPARAM>(&search));
        if (search.window && GetPropW(search.window, L"GrammarOwnedTargetReady")) break;
        Sleep(10);
    } while (GetTickCount64() < end);
    HWND edit = search.window && GetPropW(search.window, L"GrammarOwnedTargetReady") ? FindWindowExW(search.window, nullptr, L"RICHEDIT50W", nullptr) : nullptr;
    DWORD pid = 0, targetThread = edit ? GetWindowThreadProcessId(edit, &pid) : 0;
    auto callback = reinterpret_cast<HOOKPROC>(GetProcAddress(module, "GrammarOwnedBootstrapHook"));
    HHOOK hook = callback && targetThread && pid == info.dwProcessId && grammar::same_local_identity(process.get()) ?
        SetWindowsHookExW(WH_CALLWNDPROC, callback, module, targetThread) : nullptr;
    printf("thread_admission window=%d edit=%d callback=%d target_thread=%lu pid_match=%d hook=%d error=%lu\n",
        search.window != nullptr, edit != nullptr, callback != nullptr, targetThread, pid == info.dwProcessId, hook != nullptr, hook ? 0 : GetLastError());
    bool pass = false;
    if (hook) {
        DWORD_PTR result = 0; UINT message = RegisterWindowMessageW(bootstrap_message);
        bool start = SendMessageTimeoutW(edit, message, 1, 0, SMTO_ABORTIFHUNG | SMTO_BLOCK, 2000, &result) != 0;
        LONG loaded = static_cast<LONG>(reinterpret_cast<INT_PTR>(GetPropW(edit, result_property))) - 1;
        bool stop = SendMessageTimeoutW(edit, message, 2, 0, SMTO_ABORTIFHUNG | SMTO_BLOCK, 2000, &result) != 0;
        LONG remaining = static_cast<LONG>(reinterpret_cast<INT_PTR>(GetPropW(edit, result_property))) - 1;
        bool unhooked = UnhookWindowsHookEx(hook) != FALSE;
        WCHAR text[64]{};
        bool reread = SendMessageTimeoutW(edit, WM_GETTEXT, ARRAYSIZE(text), reinterpret_cast<LPARAM>(text),
            SMTO_ABORTIFHUNG | SMTO_BLOCK, 2000, &result) != 0;
        pass = start && stop && loaded == 1 && remaining == 0 && unhooked && reread && std::wstring(text) == L"synthetic target unchanged";
        printf("thread_bootstrap loaded=%ld remaining=%ld unhooked=%d source_unchanged=%d pass=%d\n", loaded, remaining,
            unhooked, std::wstring(text) == L"synthetic target unchanged", pass);
    }
    if (search.window) PostMessageW(search.window, WM_CLOSE, 0, 0);
    bool exited = WaitForSingleObject(process.get(), 5000) == WAIT_OBJECT_0;
    DWORD code = STILL_ACTIVE; GetExitCodeProcess(process.get(), &code);
    if (!exited) { TerminateProcess(process.get(), 99); WaitForSingleObject(process.get(), 3000); }
    printf("owned_target_pid=%lu natural_exit=%d exit=%lu\n", info.dwProcessId, exited, code);
    return pass && exited && code == 0;
}
int wmain(int argc, wchar_t** argv) {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount" || !grammar::same_local_identity(GetCurrentProcess())) return 2;
    if (argc == 2 && std::wstring(argv[1]) == L"--target") return target();
    HKEY machine = nullptr;
    LONG access = RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\CTF\\TIP", 0, KEY_CREATE_SUB_KEY | KEY_SET_VALUE, &machine);
    if (machine) RegCloseKey(machine);
    if (access != ERROR_ACCESS_DENIED) return 3;
    constexpr WCHAR clsid[] = L"{7DC8373F-68AE-477C-B6B8-A7398710AA35}";
    auto comPath = std::wstring(L"Software\\Classes\\CLSID\\") + clsid;
    auto tipPath = std::wstring(L"Software\\Microsoft\\CTF\\TIP\\") + clsid;
    if (exists(HKEY_CURRENT_USER, comPath) != ERROR_FILE_NOT_FOUND || exists(HKEY_LOCAL_MACHINE, comPath) != ERROR_FILE_NOT_FOUND ||
        exists(HKEY_CURRENT_USER, tipPath) != ERROR_FILE_NOT_FOUND || exists(HKEY_LOCAL_MACHINE, tipPath) != ERROR_FILE_NOT_FOUND) return 4;
    constexpr WCHAR dll[] = L"C:\\GrammarPayload\\tsf-load-probe.dll";
    HKEY key = nullptr; DWORD disposition = 0;
    LONG created = RegCreateKeyExW(HKEY_CURRENT_USER, comPath.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, &key, &disposition);
    if (created != ERROR_SUCCESS || disposition != REG_CREATED_NEW_KEY) { if(key) RegCloseKey(key); return 5; }
    HKEY server = nullptr;
    bool registeredCom = RegCreateKeyExW(key, L"InprocServer32", 0, nullptr, 0, KEY_WRITE, nullptr, &server, nullptr) == ERROR_SUCCESS;
    if (registeredCom) { registeredCom = value(server, nullptr, dll) && value(server, L"ThreadingModel", L"Apartment"); RegCloseKey(server); }
    RegCloseKey(key);
    bool clean = false; HRESULT general = E_PENDING, local = E_PENDING, activated = E_PENDING;
    LONG count = 0, remaining = -1; HRESULT unload = E_PENDING; bool bootstrapped = false;
    if (registeredCom && SUCCEEDED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) {
        {
            HMODULE module = LoadLibraryExW(dll, nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
            using Count = LONG(WINAPI*)(); using CanUnload = HRESULT(WINAPI*)();
            auto total = module ? reinterpret_cast<Count>(GetProcAddress(module, "GrammarProbeActivations")) : nullptr;
            auto live = module ? reinterpret_cast<Count>(GetProcAddress(module, "GrammarProbeActive")) : nullptr;
            auto canUnload = module ? reinterpret_cast<CanUnload>(GetProcAddress(module, "DllCanUnloadNow")) : nullptr;
            ComPtr<ITfThreadMgr> manager; ComPtr<ITfInputProcessorProfileMgr> profiles; TfClientId client = TF_CLIENTID_NULL;
            if (total && live && canUnload && SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&manager))) &&
                SUCCEEDED(manager->Activate(&client)) && SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&profiles)))) {
                ComPtr<ITfTextInputProcessorEx> direct;
                HRESULT com = CoCreateInstance(service_id, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&direct));
                printf("per_user_com_hr=%08lx\n", static_cast<unsigned long>(com));
                direct.Reset(); // No manual Activate: activation counter must come from TSF.
                HMODULE rich = LoadLibraryExW(L"Msftedit.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
                HWND top = CreateWindowExW(0, L"STATIC", L"Grammar owned service loader", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    30, 30, 600, 250, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
                HWND edit = CreateWindowExW(0, L"RICHEDIT50W", L"synthetic loader fixture", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
                    10, 10, 550, 150, top, nullptr, GetModuleHandleW(nullptr), nullptr);
                ShowWindow(top, SW_SHOW); ShowWindow(top, SW_SHOW);
                SendMessageW(edit, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
                SetForegroundWindow(top); SetFocus(edit); pump(500);
                LANGID language = LOWORD(reinterpret_cast<ULONG_PTR>(GetKeyboardLayout(0)));
                constexpr WCHAR desc[] = L"Grammar owned loader diagnosis";
                general = profiles->RegisterProfile(service_id, language, profile_id, desc, ARRAYSIZE(desc)-1, dll, ARRAYSIZE(dll)-1, 0, nullptr, 0, FALSE, 0);
                local = profiles->RegisterProfile(service_id, language, profile_id, desc, ARRAYSIZE(desc)-1, dll, ARRAYSIZE(dll)-1, 0, nullptr, 0, TRUE, TF_RP_LOCALPROCESS);
                HKL before = GetKeyboardLayout(0);
                activated = profiles->ActivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, language, service_id, profile_id, nullptr, TF_IPPMF_FORPROCESS);
                pump(100);
                count = total();
                printf("general_hr=%08lx local_hr=%08lx activate_hr=%08lx callbacks=%ld keyboard_unchanged=%d\n",
                    static_cast<unsigned long>(general), static_cast<unsigned long>(local), static_cast<unsigned long>(activated), count, before == GetKeyboardLayout(0));
                HRESULT deactivated = profiles->DeactivateProfile(TF_PROFILETYPE_INPUTPROCESSOR, language, service_id, profile_id, nullptr, TF_IPPMF_FORPROCESS);
                HRESULT released = profiles->ReleaseInputProcessor(service_id, 0);
                HRESULT removed = profiles->UnregisterProfile(service_id, language, profile_id, 0);
                ComPtr<ITfInputProcessorProfiles> old;
                HRESULT unregistered = profiles.As(&old); if (SUCCEEDED(unregistered)) unregistered = old->Unregister(service_id);
                printf("deactivate_hr=%08lx release_hr=%08lx remove_hr=%08lx unregister_hr=%08lx\n",
                    static_cast<unsigned long>(deactivated), static_cast<unsigned long>(released), static_cast<unsigned long>(removed), static_cast<unsigned long>(unregistered));
                old.Reset(); profiles.Reset(); DestroyWindow(top); manager->Deactivate(); manager.Reset(); CoFreeUnusedLibraries();
                remaining = live(); unload = canUnload();
                if (rich) FreeLibrary(rich);
            }
            if (module) { bootstrapped = target_bootstrap(module); FreeLibrary(module); }
        }
        CoUninitialize();
    }
    LONG deleted = RegDeleteTreeW(HKEY_CURRENT_USER, comPath.c_str());
    clean = deleted == ERROR_SUCCESS && exists(HKEY_CURRENT_USER, comPath) == ERROR_FILE_NOT_FOUND &&
        exists(HKEY_CURRENT_USER, tipPath) == ERROR_FILE_NOT_FOUND && exists(HKEY_LOCAL_MACHINE, tipPath) == ERROR_FILE_NOT_FOUND;
    printf("active_remaining=%ld unload_hr=%08lx registry_cleanup=%d classification=OWNED_PROCESS_LOADING_ONLY\n",
        remaining, static_cast<unsigned long>(unload), clean);
    return clean && remaining == 0 && unload == S_OK && bootstrapped ? 0 : 1;
}
#endif
