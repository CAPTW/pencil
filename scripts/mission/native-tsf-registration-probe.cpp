// Capability diagnosis only, restricted Medium owned Sandbox. No activation,
// keyboard/default changes, elevation, COM server or existing profile changes.
#include "../../src-tauri/native/apply_channel.hpp"
#include <msctf.h>
#include <wrl/client.h>
#include <cstdio>
using Microsoft::WRL::ComPtr;
static LONG exists(HKEY root, const std::wstring& path) {
    HKEY key = nullptr;
    LONG result = RegOpenKeyExW(root, path.c_str(), 0, KEY_READ, &key);
    if (key) RegCloseKey(key);
    return result;
}
int wmain() {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount" || !grammar::same_local_identity(GetCurrentProcess())) return 2;
    HANDLE raw = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw)) return 3;
    grammar::ChannelHandle token(raw); TOKEN_ELEVATION elevation{}; DWORD size = 0;
    if (!GetTokenInformation(raw, TokenElevation, &elevation, sizeof(elevation), &size) || elevation.TokenIsElevated) return 4;
    // Do not call registration if this token could write the machine TIP root.
    HKEY machine = nullptr;
    LONG machineAccess = RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\CTF\\TIP", 0,
        KEY_CREATE_SUB_KEY | KEY_SET_VALUE, &machine);
    if (machine) RegCloseKey(machine);
    printf("machine_write_access=%ld elevated=0\n", machineAccess);
    if (machineAccess != ERROR_ACCESS_DENIED) return 5;
    if (FAILED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) return 6;
    bool pass = false;
    {
        ComPtr<ITfInputProcessorProfileMgr> profiles;
        if (FAILED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
            IID_PPV_ARGS(&profiles)))) return 7;
        GUID service{}, profile{}; WCHAR guid[40]{};
        if (FAILED(CoCreateGuid(&service)) || FAILED(CoCreateGuid(&profile)) || !StringFromGUID2(service, guid, 40)) return 8;
        auto path = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\TIP\\") + guid;
        if (exists(HKEY_LOCAL_MACHINE, path) != ERROR_FILE_NOT_FOUND || exists(HKEY_CURRENT_USER, path) != ERROR_FILE_NOT_FOUND) return 9;
        constexpr WCHAR description[] = L"Grammar owned registration capability probe";
        LANGID language = LOWORD(reinterpret_cast<ULONG_PTR>(GetKeyboardLayout(0)));
        HRESULT registered = profiles->RegisterProfile(service, language, profile, description,
            ARRAYSIZE(description) - 1, L"", 0, 0, nullptr, 0, FALSE, 0);
        LONG userResult = exists(HKEY_CURRENT_USER, path), machineResult = exists(HKEY_LOCAL_MACHINE, path);
        printf("register_hr=%08lx user_key=%ld machine_key=%ld enabled_by_default=0\n",
            static_cast<unsigned long>(registered), userResult, machineResult);
        // Only this new, previously absent random GUID may be unregistered.
        HRESULT removed = profiles->UnregisterProfile(service, language, profile, 0);
        ComPtr<ITfInputProcessorProfiles> legacy;
        HRESULT serviceRemoved = profiles.As(&legacy);
        if (SUCCEEDED(serviceRemoved)) serviceRemoved = legacy->Unregister(service);
        LONG finalUser = exists(HKEY_CURRENT_USER, path), finalMachine = exists(HKEY_LOCAL_MACHINE, path);
        printf("unregister_hr=%08lx service_unregister_hr=%08lx final_user=%ld final_machine=%ld\n",
            static_cast<unsigned long>(removed), static_cast<unsigned long>(serviceRemoved), finalUser, finalMachine);
        pass = finalUser == ERROR_FILE_NOT_FOUND && finalMachine == ERROR_FILE_NOT_FOUND;
        HRESULT local = profiles->RegisterProfile(service, language, profile, description,
            ARRAYSIZE(description) - 1, L"", 0, 0, nullptr, 0, FALSE, TF_RP_LOCALPROCESS);
        HRESULT localRemoved = profiles->UnregisterProfile(service, language, profile, 0);
        printf("local_process_register_hr=%08lx local_remove_hr=%08lx\n",
            static_cast<unsigned long>(local), static_cast<unsigned long>(localRemoved));
        pass = pass && exists(HKEY_CURRENT_USER, path) == ERROR_FILE_NOT_FOUND && exists(HKEY_LOCAL_MACHINE, path) == ERROR_FILE_NOT_FOUND;
        printf("classification=%s cleanup=%d\n", SUCCEEDED(registered) ? "USER_REGISTRATION_AVAILABLE_NOT_ACTIVATED" : "REGISTRATION_UNAVAILABLE_WITH_CURRENT_AUTHORITY", pass);
    }
    CoUninitialize(); return pass ? 0 : 1;
}
