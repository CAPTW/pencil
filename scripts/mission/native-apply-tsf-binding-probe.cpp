// Owned Sandbox: actual read session and edit notifications, no invented grants.
#include <windows.h>
#include <richedit.h>
#include <cstdio>
#include <thread>
#include "../../src-tauri/native/tsf_capture.hpp"
#include "../../src-tauri/native/target_apply_focus.hpp"
using Microsoft::WRL::ComPtr;
static void pump(DWORD ms) {
    auto end = GetTickCount64() + ms;
    do {
        MSG msg;
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) { TranslateMessage(&msg); DispatchMessageW(&msg); }
        Sleep(5);
    } while (GetTickCount64() < end);
}
int wmain() {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount") return 2;
    if (FAILED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) return 3;
    int failures = 0;
    {
        ComPtr<ITfThreadMgr> manager; TfClientId client = 0;
        if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&manager))) ||
            FAILED(manager->Activate(&client))) return 4;
        auto library = LoadLibraryW(L"msctf.dll");
        using SetScope = HRESULT(WINAPI*)(HWND, InputScope);
        auto setScope = reinterpret_cast<SetScope>(GetProcAddress(library, "SetInputScope"));
        if (!setScope) return 5;
        LoadLibraryW(L"Msftedit.dll");
        HWND top = CreateWindowExW(0, L"STATIC", L"Grammar owned TSF capture", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            30, 30, 600, 250, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
        HWND edit = CreateWindowExW(0, L"RICHEDIT50W", L"hello world", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
            10, 10, 550, 150, top, nullptr, GetModuleHandleW(nullptr), nullptr);
        HWND other = CreateWindowExW(0, L"RICHEDIT50W", L"untouched", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
            10, 165, 550, 30, top, nullptr, GetModuleHandleW(nullptr), nullptr);
        if (!top || !edit || !other) return 6;
        // The helper starts hidden; explicitly show only its owned guest editor.
        ShowWindow(top, SW_SHOW); ShowWindow(top, SW_SHOW);
        SendMessageW(edit, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
        SetForegroundWindow(top); SetFocus(edit); pump(500);
        ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> context;
        if (FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context) return 7;
        for (int scenario = 0; scenario < 25; ++scenario) {
            SetForegroundWindow(top); SetFocus(edit);
            SendMessageW(edit, EM_SETREADONLY, FALSE, 0);
            SendMessageW(edit, EM_SETPASSWORDCHAR, 0, 0);
            if (FAILED(setScope(edit, IS_TEXT))) return 8;
            const std::wstring original = L"\ud55c\uae00 \xd83d\xde00 world";
            SetWindowTextW(edit, original.c_str());
            SendMessageW(edit, EM_SETSEL, 0, -1);
            CHARFORMAT2W format{}; format.cbSize = sizeof(format); format.dwMask = CFM_PROTECTED | CFM_HIDDEN;
            SendMessageW(edit, EM_SETCHARFORMAT, SCF_SELECTION, reinterpret_cast<LPARAM>(&format));
            SendMessageW(edit, EM_SETSEL, 6, 11); pump(50);
            auto binding = std::make_unique<grammar::TsfDocumentBinding>(context.Get(), scenario == 9 ? other : edit, scenario != 14);
            if (scenario == 6) SendMessageW(edit, EM_SETPASSWORDCHAR, L'*', 0);
            if (scenario == 7) SendMessageW(edit, EM_SETREADONLY, TRUE, 0);
            if (scenario == 8 && FAILED(setScope(edit, IS_PASSWORD))) return 9;
            if (scenario == 19 || scenario == 20) {
                format.dwEffects = scenario == 19 ? CFE_PROTECTED : CFE_HIDDEN;
                SendMessageW(edit, EM_SETCHARFORMAT, SCF_SELECTION, reinterpret_cast<LPARAM>(&format));
            }
            pump(50);
            auto capture = new grammar::TsfCaptureSession(binding->state, [] { return true; }); // Trusted in-process fixture.
            HRESULT completed = E_PENDING;
            HRESULT request = context->RequestEditSession(client, capture, TF_ES_ASYNC | TF_ES_READ, &completed);
            for (int i = 0; i < 100 && !capture->called && SUCCEEDED(request); ++i) pump(10);
            bool denied = (scenario >= 6 && scenario <= 9) || scenario == 14 || scenario == 19 || scenario == 20;
            bool pass = capture->called && (denied ? !capture->captured && capture->text_reads == 0 :
                capture->captured && capture->source == original && capture->start == 6 && capture->end == 11);
            printf("capture case=%d pass=%d captured=%d reads=%u notifications=%u\n", scenario, pass, capture->captured, capture->text_reads, binding->state->notifications);
            printf("eligibility_stage=%u scope_type=%u enabled=%d\n", binding->state->eligibility_stage, binding->state->scope_type, binding->state->enabled);
            if (!pass) ++failures;
            std::atomic<bool> review_ready{false}, review_stop{false}, review_done{false};
            std::thread review;
            if (scenario == 23 && capture->captured) {
                review = std::thread([&] {
                    HWND widget = CreateWindowW(L"STATIC", L"Owned review", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                        100,100,400,200,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
                    ShowWindow(widget,SW_SHOW); SetForegroundWindow(widget); review_ready=true;
                    while(!review_stop) pump(10);
                    DestroyWindow(widget); review_done=true;
                });
                for(int i=0;i<100&&!review_ready;++i) pump(10);
                pump(100);
                printf("review_focus revision=%llu valid=%d notifications=%u transitions=%u\n",binding->state->revision,
                    capture->grant->valid.load(),binding->state->notifications,binding->state->eligibility_transitions);
            }
            if (!denied && capture->captured) {
                auto revision = capture->grant->revision.load();
                if (scenario == 1) SetWindowTextW(edit, L"application changed");
                if (scenario == 2) SendMessageW(edit, EM_SETSEL, 0, 2);
                if (scenario == 3) {
                    SetWindowTextW(edit, L"transient"); SetWindowTextW(edit, original.c_str());
                    SendMessageW(edit, EM_SETSEL, 6, 11);
                }
                if (scenario == 4) { SendMessageW(edit, EM_SETSEL, 0, 2); SendMessageW(edit, EM_SETSEL, 6, 11); }
                if (scenario == 22) {
                    if (FAILED(setScope(edit, IS_PASSWORD)) || FAILED(setScope(edit, IS_TEXT))) return 11;
                }
                if (scenario == 24) {
                    SetFocus(other);
                    if(grammar::activate_apply_target(edit,GetCurrentProcessId()) || GetFocus()!=other) ++failures;
                    SetFocus(edit); // Returning to the same field cannot revive the old grant.
                }
                if (scenario == 12) {
                    auto newer = new grammar::TsfCaptureSession(binding->state, [] { return true; });
                    request = context->RequestEditSession(client, newer, TF_ES_ASYNC | TF_ES_READ, &completed);
                    for (int i = 0; i < 100 && !newer->called && SUCCEEDED(request); ++i) pump(10);
                    if (!newer->captured) ++failures;
                    newer->Release();
                }
                pump(30);
                if(scenario==23) {
                    HWND previous=GetForegroundWindow();
                    if(grammar::activate_apply_target(edit,0) || GetForegroundWindow()!=previous) ++failures;
                    bool activated=grammar::activate_apply_target(edit,GetCurrentProcessId());
                    printf("review_reactivate=%d focus_field=%d\n",activated,GetFocus()==edit);
                    if(!activated) ++failures;
                }
                auto apply = new grammar::TsfApplySession(context.Get(), capture->source, capture->start, capture->end,
                    L"Grammar", revision, capture->grant);
                request = context->RequestEditSession(client, apply, TF_ES_ASYNC | TF_ES_READWRITE, &completed);
                bool queued = !apply->called && completed == TF_S_ASYNC;
                if (scenario == 5) binding->disable();
                if (scenario == 10) binding.reset();
                if (scenario == 13) SendMessageW(edit, EM_SETREADONLY, TRUE, 0);
                if (scenario == 15 && FAILED(setScope(edit, IS_PASSWORD))) return 10;
                if (scenario == 16) {
                    printf("password before called=%d mask=%lld\n", apply->called, static_cast<long long>(SendMessageW(edit, EM_GETPASSWORDCHAR, 0, 0)));
                    SendMessageW(edit, EM_SETPASSWORDCHAR, L'*', 0);
                    printf("password after called=%d mask=%lld\n", apply->called, static_cast<long long>(SendMessageW(edit, EM_GETPASSWORDCHAR, 0, 0)));
                }
                if (scenario == 17) SetWindowTextW(edit, L"queued application change");
                if (scenario == 18) SendMessageW(edit, EM_SETSEL, 0, 2);
                if (scenario == 21) {
                    format.dwEffects = CFE_PROTECTED;
                    SendMessageW(edit, EM_SETCHARFORMAT, SCF_SELECTION, reinterpret_cast<LPARAM>(&format));
                }
                for (int i = 0; i < 100 && !apply->called && SUCCEEDED(request); ++i) pump(10);
                const bool shouldApply = scenario == 0 || scenario == 11 || scenario == 23;
                std::wstring expected = shouldApply ? original.substr(0, 6) + L"Grammar" : scenario == 1 ? L"application changed" :
                    scenario == 17 ? L"queued application change" : original;
                WCHAR actual[128]{}; GetWindowTextW(edit, actual, 128);
                pass = queued && apply->called && std::wstring(actual) == expected &&
                    apply->mutation_calls == (shouldApply ? 1u : 0u) &&
                    apply->text_reads == (shouldApply ? 2u : 0u) &&
                    apply->result == (shouldApply ? grammar::ApplyResult::applied : grammar::ApplyResult::rejected);
                printf("apply case=%d pass=%d mutations=%u stage=%u revision=%llu current=%llu notifications=%u transitions=%u\n", scenario, pass,
                    apply->mutation_calls, apply->diagnostic_stage, revision, binding ? binding->state->revision : 0,
                    binding ? binding->state->notifications : 0, binding ? binding->state->eligibility_transitions : 0);
                if (!pass) ++failures;
                printf("apply_text_reads=%u eligibility_stage=%u\n", apply->text_reads, binding ? binding->state->eligibility_stage : 0);
                apply->Release();
                if (scenario == 11) {
                    auto replay = new grammar::TsfApplySession(context.Get(), capture->source, 6, 11, L"Grammar", revision, capture->grant);
                    request = context->RequestEditSession(client, replay, TF_ES_ASYNC | TF_ES_READWRITE, &completed);
                    for (int i = 0; i < 100 && !replay->called && SUCCEEDED(request); ++i) pump(10);
                    GetWindowTextW(edit, actual, 128);
                    pass = replay->called && replay->mutation_calls == 0 && std::wstring(actual) == expected;
                    printf("replay pass=%d\n", pass); if (!pass) ++failures;
                    replay->Release();
                }
            }
            if(review.joinable()) { review_stop=true; while(!review_done) pump(10); review.join(); }
            capture->Release();
            if (binding) {
                std::weak_ptr<grammar::TsfBindingState> lifetime = binding->state;
                binding.reset();
                if (!lifetime.expired()) ++failures;
            }
            WCHAR untouched[128]{}; GetWindowTextW(other, untouched, 128);
            if (std::wstring(untouched) != L"untouched") ++failures;
        }
        DestroyWindow(top); manager->Deactivate(); FreeLibrary(library);
    }
    CoUninitialize(); printf("failures=%d\n", failures); return failures ? 1 : 0;
}
