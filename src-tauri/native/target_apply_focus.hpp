#pragma once
#include <windows.h>
namespace grammar {
// Explicit Apply only. Scope metadata may be unavailable while RichEdit is
// inactive. Activate the exact target, then re-run every TSF privacy/source/
// revision/range check. Never select a range or override a different field.
inline bool activate_apply_target(HWND field, DWORD broker) {
    DWORD process=0;
    if (!IsWindow(field) || GetWindowThreadProcessId(field,&process)!=GetCurrentThreadId() || process!=GetCurrentProcessId()) return false;
    HWND top=GetAncestor(field,GA_ROOT), foreground=GetForegroundWindow();
    if (!top || !IsWindowVisible(top) || !IsWindowEnabled(top) || IsIconic(top)) return false;
    DWORD owner=0; if(!foreground || !GetWindowThreadProcessId(foreground,&owner)) return false;
    if(foreground!=top && owner!=broker) return false;
    HWND focused=GetFocus();
    if(focused && focused!=field && focused!=top) return false;
    if(foreground!=top && !SetForegroundWindow(top)) return false;
    if(GetForegroundWindow()!=top) return false;
    focused=GetFocus();
    if(focused && focused!=field && focused!=top) return false;
    if(focused!=field) SetFocus(field);
    return GetFocus()==field && GetForegroundWindow()==top;
}
}
