"""Writer transaction core. Not a registered UNO service or an enabled adapter.

A deployment entry point must supply authenticated transport and explicit document
opt-in. Formatting expansion is opt-in and disabled in product deployments until qualified.
"""
import ctypes
import json
import hashlib
import math
import ntpath
import os
import secrets
import threading
import time
import uno
import unohelper
from com.sun.star.lang import XComponent
from com.sun.star.task import XJob
from com.sun.star.document import XUndoAction, UndoFailedException, XDocumentEventListener
from com.sun.star.awt import XCallback
from com.sun.star.util import XModifyListener
from com.sun.star.view import XSelectionChangeListener
from com.sun.star.lang.SystemDependent import SYSTEM_WIN32
from com.sun.star.beans.PropertyAttribute import READONLY


class Refused(Exception):
    pass


FORMATTED_CHARACTERS = ('CharWeight', 'CharWeightAsian', 'CharWeightComplex', 'CharColor')

# DOCX RGB imports materialize these color aliases. Only non-theme, opaque RGB
# and the matching original-RGB roundtrip property are accepted, never a generic
# interop bag or an arbitrary complex/theme color.
RGB_ALIASES = ('CharColorTheme', 'CharColorTintOrShade', 'CharComplexColor',
    'CharInteropGrabBag', 'CharTransparence')


def rgb_metadata(portion):
    color = portion.CharColor
    if (portion.CharColorTheme != -1 or portion.CharColorTintOrShade != 0
            or portion.CharTransparence != 0):
        raise Refused('unsupported_theme_or_transparency')
    complex_color = portion.CharComplexColor
    # Pinned26.2.4.2 model::ColorType:0=Unused,1=RGB. An unused wrapper
    # does not represent CharColor; never resolve it as if it were RGB.
    if complex_color is not None and (complex_color.getType() not in (0, 1)
            or complex_color.getThemeColorType() != -1
            or (complex_color.getType() == 1 and complex_color.resolveColor(None) != color)):
        raise Refused('unsupported_complex_color')
    bag = portion.CharInteropGrabBag
    if not bag:
        return ()
    if (len(bag) != 1 or bag[0].Name != 'CharThemeOriginalColor'
            or not isinstance(bag[0].Value, str) or not 0 <= color <= 0xFFFFFF
            or bag[0].Value.upper() != format(color, '06X')):
        raise Refused('unsupported_color_interop')
    return (bag[0].Value,)


def effective_character_format(portion, paragraph):
    result=[]
    for name in FORMATTED_CHARACTERS:
        value=portion.getPropertyValue(name)
        state=portion.getPropertyState(name).value
        # Writer can move identical run-local direct formatting to a direct
        # paragraph character property after whole-paragraph replacement.
        # This is NOT style inheritance: require an exact directly-set parent.
        if (state=='DEFAULT_VALUE' and paragraph.getPropertyState(name).value=='DIRECT_VALUE'
                and paragraph.getPropertyValue(name)==value):
            state='DIRECT_VALUE'
        result.append((name,value,state))
    return tuple(result)


def plain_metadata(doc, allow_uniform_format=False, allow_mixed_format=False):
    if (doc.TextSections.Count or doc.TextTables.Count or doc.TextFrames.Count or doc.DrawPage.Count
            or doc.TextFields.createEnumeration().hasMoreElements() or doc.Bookmarks.hasElements()
            or doc.Redlines.Count or doc.RecordChanges):
        raise Refused('unsupported_structure')
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(16385, True)
    if doc.Text.compareRegionEnds(cursor, doc.Text.End) != 0:
        raise Refused('body_bounds')
    if cursor.getPropertyState('CharHidden').value == 'AMBIGUOUS_VALUE' or cursor.CharHidden:
        raise Refused('hidden_text')
    # The custom Undo action restores text, not arbitrary rich formatting.
    # Inspect actual portions, not aggregate cursor property states: Writer can
    # report DEFAULT_VALUE there even after direct formatting was assigned.
    style = doc.StyleFamilies.getByName('ParagraphStyles').getByName('Standard')
    characters = ('CharFontName', 'CharFontNameAsian', 'CharFontNameComplex',
        'CharHeight', 'CharHeightAsian', 'CharHeightComplex',
        'CharWeight', 'CharWeightAsian', 'CharWeightComplex',
        'CharPosture', 'CharPostureAsian', 'CharPostureComplex',
        'CharColor', 'CharBackColor', 'CharUnderline', 'CharStrikeout',
        'CharEscapement', 'CharKerning', 'CharAutoKerning', 'CharScaleWidth',
        'CharCaseMap', 'CharRelief', 'CharShadowed', 'CharContoured', 'CharRotation', 'CharEmphasis')
    paragraphs = ('ParaAdjust', 'ParaLeftMargin', 'ParaRightMargin', 'ParaTopMargin',
        'ParaBottomMargin', 'ParaFirstLineIndent')
    defaults = {name: style.getPropertyValue(name) for name in characters + paragraphs}
    enumeration = doc.Text.createEnumeration()
    paragraph_count = portion_count = 0
    uniform = None
    paragraph_format = None
    color_metadata = None
    runs = []
    paragraph_rows = []
    paragraph_offset = 0
    previous_end = None
    while enumeration.hasMoreElements():
        paragraph_count += 1
        if allow_uniform_format and allow_mixed_format != 'paragraphs' and paragraph_count != 1:
            raise Refused('formatted_single_paragraph_only')
        if paragraph_count > 256:
            raise Refused('paragraph_bounds')
        paragraph = enumeration.nextElement()
        if paragraph.ParaStyleName != 'Standard' or paragraph.NumberingStyleName:
            raise Refused('unsupported_paragraph_style')
        for name in paragraphs:
            if allow_uniform_format and name == 'ParaAdjust':
                paragraph_format = (paragraph.getPropertyValue(name), paragraph.getPropertyState(name).value)
                continue
            if paragraph.getPropertyValue(name) != defaults[name]:
                raise Refused('unsupported_format_' + name)
        portions = paragraph.createEnumeration()
        if allow_mixed_format == 'paragraphs':
            if paragraph_count > 1:
                gap = doc.Text.createTextCursorByRange(previous_end)
                gap.gotoRange(paragraph.Start, True)
                separator = gap.String
                if separator not in ('\n', '\r', '\r\n'):
                    raise Refused('unsupported_paragraph_separator')
                runs.append((len(separator), (), ('paragraph_break', separator)))
                paragraph_offset += len(separator)
            paragraph_rows.append((paragraph_offset, paragraph_format))
            previous_end = paragraph.End
        while portions.hasMoreElements():
            portion_count += 1
            if portion_count > 1024:
                raise Refused('portion_bounds')
            portion = portions.nextElement()
            if portion.TextPortionType != 'Text' or portion.CharStyleName or portion.HyperLinkURL or portion.CharHidden:
                raise Refused('unsupported_text_portion')
            for property_info in portion.getPropertySetInfo().getProperties():
                name = property_info.Name
                if name.startswith('Char') and not (property_info.Attributes & READONLY):
                    if allow_uniform_format and name in FORMATTED_CHARACTERS + RGB_ALIASES:
                        continue
                    if portion.getPropertyState(name).value != 'DEFAULT_VALUE':
                        raise Refused('unsupported_character_format_' + name)
            if allow_uniform_format:
                rgb = rgb_metadata(portion)
                if not allow_mixed_format and color_metadata is not None and rgb != color_metadata:
                    raise Refused('mixed_color_metadata')
                color_metadata = rgb
                current = effective_character_format(portion, paragraph)
                if any(state not in ('DEFAULT_VALUE', 'DIRECT_VALUE') for _, _, state in current):
                    raise Refused('ambiguous_format')
                if not allow_mixed_format and uniform is not None and current != uniform:
                    raise Refused('mixed_character_format')
                uniform = current
                length = len(portion.String)
                paragraph_offset += length
                if length:
                    if runs and runs[-1][1:] == (current, rgb):
                        runs[-1] = (runs[-1][0] + length, current, rgb)
                    else:
                        runs.append((length, current, rgb))
                elif allow_mixed_format == 'paragraphs':
                    # Empty paragraphs retain a typing format even without text.
                    # DOCX may also append a redundant empty portion to a
                    # nonempty run. Canonicalize it exactly like adjacent text.
                    if not runs or runs[-1][1:] != (current, rgb):
                        runs.append((0, current, rgb))
            for name in characters:
                if allow_uniform_format and name in FORMATTED_CHARACTERS:
                    continue
                if portion.getPropertyValue(name) != defaults[name]:
                    raise Refused('unsupported_format_' + name)

    if allow_uniform_format:
        if uniform is None or paragraph_format is None:
            raise Refused('empty_format')
        effective_defaults = tuple((name, value if isinstance(value, (str, int, float, bool)) else value.value)
            for name, value in sorted(defaults.items()))
        if allow_mixed_format:
            # Canonicalize equivalent adjacent portions. UNO may split/merge
            # portions on replacement and Undo without changing their format.
            return (tuple(runs), ('paragraphs', tuple(paragraph_rows))
                if allow_mixed_format == 'paragraphs' else paragraph_format, effective_defaults)
        return (uniform, paragraph_format, effective_defaults, color_metadata)
    return None


def mixed_selection(doc, selected, layout, allow_cross=False):
    prefix = doc.Text.createTextCursor()
    prefix.gotoStart(False)
    prefix.gotoRange(selected.Start, True)
    start, length = len(prefix.String), len(selected.String)
    if not length or sum(run[0] for run in layout[0]) != len(doc.Text.String):
        raise Refused('mixed_range_bounds')
    offset = 0
    chosen = None
    covered = 0
    paragraph_format = layout[1]
    if layout[1][0] == 'paragraphs':
        paragraph_format = next(value for offset, value in reversed(layout[1][1]) if offset <= start)
    for size, characters, rgb in layout[0]:
        overlap = max(0, min(offset + size, start + length) - max(offset, start))
        if overlap:
            if not characters:
                raise Refused('selection_crosses_paragraph_boundary')
            current = (characters, paragraph_format, layout[2], rgb)
            if not allow_cross and chosen is not None and current != chosen:
                raise Refused('selection_crosses_format_boundary')
            if chosen is None:
                chosen = current
            covered += overlap
        offset += size
    if covered != length:
        raise Refused('mixed_range_bounds')
    return start, length, chosen


def selected_layout_runs(layout, start, length):
    """Bounded original character formats for range-only Undo; never delimiters."""
    result, offset, covered = [], 0, 0
    for size, characters, rgb in layout[0]:
        overlap = max(0, min(offset + size, start + length) - max(offset, start))
        if overlap:
            if not characters:
                raise Refused('selection_crosses_paragraph_boundary')
            result.append((overlap, characters, rgb))
            covered += overlap
        offset += size
    if start < 0 or length <= 0 or covered != length:
        raise Refused('mixed_range_bounds')
    return tuple(result)


def replaced_layout(layout, start, length, replacement_length, allow_cross=False):
    """Replace with first selected style; original range formats retained for Undo."""
    selected = selected_layout_runs(layout, start, length)
    chosen = selected[0][1:]
    if not allow_cross and any(run[1:] != chosen for run in selected):
        raise Refused('selection_crosses_format_boundary')
    if replacement_length <= 0:
        raise Refused('mixed_range_bounds')
    output, offset, inserted = [], 0, False
    def append(size, characters, rgb):
        if characters and output and output[-1][1:] == (characters, rgb):
            output[-1] = (output[-1][0] + size, characters, rgb)
        else:
            output.append((size, characters, rgb))
    for size, characters, rgb in layout[0]:
        end = offset + size
        overlap = max(0, min(end, start + length) - max(offset, start))
        if not overlap:
            append(size, characters, rgb)
        else:
            if offset < start:
                append(start - offset, characters, rgb)
            if not inserted:
                append(replacement_length, *chosen)
                inserted = True
            if end > start + length:
                append(end - start - length, characters, rgb)
        offset = end
    paragraph_format = layout[1]
    if paragraph_format[0] == 'paragraphs':
        paragraph_format = ('paragraphs', tuple((offset + (replacement_length - length if offset > start else 0), value)
            for offset, value in paragraph_format[1]))
    return (tuple(output), paragraph_format, layout[2])


def paragraph_span_plan(body, layout, start, length, replacement):
    """Keep paragraph topology; map each nonempty selection part to one draft part."""
    if layout[1][0] != 'paragraphs' or length <= 0 or start < 0 or start + length > len(body):
        raise Refused('span_bounds')
    if any(c in replacement for c in '\u2028\u2029') or '\r' in replacement.replace('\r\n', ''):
        raise Refused('span_separator')
    breaks, offset = [], 0
    for size, chars, rgb in layout[0]:
        if not chars:
            if body[offset:offset+size] not in ('\n', '\r', '\r\n'):
                raise Refused('span_separator')
            breaks.append((offset, offset+size))
        offset += size
    if offset != len(body): raise Refused('span_bounds')
    inside = [(a,b) for a,b in breaks if a < start+length and b > start]
    if not inside or len(inside) > 7 or any(a < start or b > start+length for a,b in inside):
        raise Refused('span_bounds')
    bounds = [start] + [b for a,b in inside]
    ends = [a for a,b in inside] + [start+length]
    parts = replacement.replace('\r\n', '\n').split('\n')
    if len(parts) != len(bounds): raise Refused('span_paragraph_count')
    plans=[]
    expected_body, expected_layout = body, layout
    for begin,end,part in reversed(list(zip(bounds,ends,parts))):
        if not part or begin == end:
            if part or begin != end: raise Refused('span_empty_paragraph_changed')
            continue
        runs=selected_layout_runs(layout,begin,end-begin)
        para=next(value for pos,value in reversed(layout[1][1]) if pos<=begin)
        fmt=(runs[0][1],para,layout[2],runs[0][2])
        plans.append((begin,end-begin,part,fmt,runs))
        expected_layout=replaced_layout(expected_layout,begin,end-begin,len(part),True)
        expected_body=expected_body[:begin]+part+expected_body[end:]
    if len(plans)<2: raise Refused('span_nonempty_parts')
    return plans,expected_body,expected_layout


def span_cursor(doc, selected, offset, length):
    cursor=doc.Text.createTextCursorByRange(selected.Start)
    for stage,target in enumerate((offset,length)):
        for _ in range(4097):
            measured=len(cursor.String)
            if measured==target: break
            if measured>target or not cursor.goRight(1,True): raise Refused('span_cursor_bounds')
        else: raise Refused('span_cursor_bounds')
        if stage==0:
            cursor.collapseToEnd()
    return cursor


def apply_paragraph_span(owner, review, replacement, deadline):
    wrote=False
    failure=None
    outcome=None
    manager=owner.document.getUndoManager()
    entered=False
    try:
        owner.validate(review,deadline)
        body=body_snapshot(owner.document,True,owner.allow_mixed_format)
        prefix=owner.document.Text.createTextCursor()
        prefix.gotoStart(False);prefix.gotoRange(review['range'].Start,True)
        start=len(prefix.String)
        plans,expected_body,expected_layout=paragraph_span_plan(body,review['layout'],start,len(review['source']),replacement)
        cursors=[span_cursor(owner.document,review['range'],a-start,n) for a,n,_,_,_ in plans]
        owner.validate(review,deadline)
        manager.enterUndoContext('Grammar reviewed paragraph replacement');entered=True
        owner.validate(review,deadline)
        if manager.isLocked(): raise Refused('undo_owned_elsewhere')
        current_body,current_layout=body,review['layout']
        current_hash=document_fingerprint(owner.document,True,owner.allow_mixed_format)
        expected_revision=(owner.revision,owner.selection_revision)
        for (offset,length,part,fmt,runs),cursor in zip(plans,cursors):
            owner.permission(review['epoch']);owner.deadline(deadline)
            if (owner.revision,owner.selection_revision)!=expected_revision: raise Refused('span_revision_changed')
            if time.monotonic()>=review['expires']: raise Refused('expired_review')
            selected=owner.selected()
            text=owner.document.Text
            if text.compareRegionStarts(selected,review['range']) or text.compareRegionEnds(selected,review['range']):
                raise Refused('span_selection_changed')
            # Review widget owns foreground during explicit Accept, as in the
            # single-range path. Exact bound document/controller/range still checked.
            owner.check_document(owner.document)
            if document_fingerprint(owner.document,True,owner.allow_mixed_format)!=current_hash:
                raise Refused('span_document_changed')
            original=cursor.String
            if original!=current_body[offset:offset+length]: raise Refused('span_source_changed')
            steps=movement_to(owner.document,cursor.Start)
            count=movement_to(owner.document,cursor.End)-steps
            next_layout=replaced_layout(current_layout,offset,length,len(part),True)
            next_body=current_body[:offset]+part+current_body[offset+length:]
            next_hash=fingerprint(json.dumps((next_body,next_layout),ensure_ascii=True,separators=(',',':')))
            manager.lock()
            try:
                owner.document.disableSetModified()
                try:
                    owner.permission(review['epoch']);owner.deadline(deadline)
                    if (owner.revision,owner.selection_revision)!=expected_revision: raise Refused('span_revision_changed')
                    owner.check_document(owner.document,own_notification_batch=True)
                    selected=owner.selected()
                    if time.monotonic()>=review['expires'] or owner.document.Text.compareRegionStarts(selected,review['range']) or owner.document.Text.compareRegionEnds(selected,review['range']):
                        raise Refused('span_review_changed')
                    if cursor.String!=original or document_fingerprint(owner.document,True,owner.allow_mixed_format)!=current_hash:
                        raise Refused('span_document_changed')
                    wrote=True
                    cursor.String=part
                    restore_uniform_format(cursor,fmt)
                finally: owner.document.enableSetModified()
            finally: manager.unlock()
            if document_fingerprint(owner.document,True,owner.allow_mixed_format)!=next_hash:
                raise Refused('span_postwrite_mismatch')
            action=ReplacementUndo(owner.document,cursor,original,part,steps,count,current_hash,next_hash,fmt,owner.allow_mixed_format,runs)
            current_body,current_layout,current_hash=next_body,next_layout,next_hash
            expected_revision=(owner.revision,owner.selection_revision)
            manager.addUndoAction(action)
            if (owner.revision,owner.selection_revision)!=expected_revision: raise Refused('span_revision_changed')
        owner.document.setModified(True)
        if current_body!=expected_body or current_layout!=expected_layout or document_fingerprint(owner.document,True,owner.allow_mixed_format)!=current_hash:
            raise Refused('span_postwrite_mismatch')
        outcome=dict(status='APPLIED',undo=False)
        return outcome,wrote
    except Exception as error:
        failure=str(error)
        return dict(status='UNKNOWN_CHANGED' if wrote else 'REJECTED',reason=failure),wrote
    finally:
        if entered:
            try:
                if wrote: owner.document.setModified(True)
                manager.leaveUndoContext()
                if outcome is not None:
                    outcome['undo']=bool(manager.isUndoPossible())
                if wrote:
                    owner.permission(review['epoch'])
                    if document_fingerprint(owner.document,True,owner.allow_mixed_format)!=current_hash:
                        raise Refused('span_changed_after_context')
            except Exception as error:
                return dict(status='UNKNOWN_CHANGED' if wrote else 'REJECTED',reason=(failure+'; ' if failure else '')+str(error)),wrote


def restore_selected_runs(doc, selected, runs, formatting):
    # UNO cursor units differ from Python characters (e.g. astral text).
    # Measure actual selected strings; never guess a UTF-16 offset.
    cursor = doc.Text.createTextCursorByRange(selected.Start)
    for length, characters, rgb in runs:
        cursor.collapseToEnd()
        for _ in range(4097):
            measured = len(cursor.String)
            if measured == length:
                break
            if measured > length or not cursor.goRight(1, True):
                raise Refused('unrepresentable_format_range')
        else:
            raise Refused('format_range_bounds')
        restore_uniform_format(cursor, (characters, formatting[1], formatting[2], rgb))
    if doc.Text.compareRegionEnds(cursor, selected.End) != 0:
        raise Refused('format_range_bounds')


def restore_uniform_format(selected, formatting):
    # Only homogeneous character properties admitted by metadata are restored.
    # Paragraph topology is unchanged: formatted replacements cannot contain breaks.
    for name, value, state in formatting[0]:
        if selected.getPropertyValue(name) == value:
            continue
        # A portion DEFAULT_VALUE can inherit a run value, not the document
        # default. After replacement that run may no longer exist. Restore the
        # measured effective value instead of resetting to a different default.
        selected.setPropertyValue(name, value)
    if formatting[3]:
        item = uno.createUnoStruct('com.sun.star.beans.PropertyValue')
        item.Name, item.Value = 'CharThemeOriginalColor', formatting[3][0]
        uno.invoke(selected, 'setPropertyValue', ('CharInteropGrabBag',
            uno.Any('[]com.sun.star.beans.PropertyValue', (item,))))
    elif selected.CharInteropGrabBag:
        selected.setPropertyToDefault('CharInteropGrabBag')


def body_snapshot(doc, allow_uniform_format=False, allow_mixed_format=False):
    plain_metadata(doc, allow_uniform_format, allow_mixed_format)
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(16385, True)
    if doc.Text.compareRegionEnds(cursor, doc.Text.End) != 0:
        raise Refused('body_bounds')
    value = cursor.String
    if len(value) > 16384:
        raise Refused('body_bounds')
    return value


def document_fingerprint(doc, allow_uniform_format=False, allow_mixed_format=False):
    body = body_snapshot(doc, allow_uniform_format, allow_mixed_format)
    if allow_uniform_format:
        body = json.dumps((body, plain_metadata(doc, True, allow_mixed_format)), ensure_ascii=True, separators=(',', ':'))
    return fingerprint(body)


def fingerprint(value):
    return hashlib.sha256(value.encode('utf-8')).digest()


def movement_to(doc, target):
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    for steps in range(16385):
        if doc.Text.compareRegionStarts(cursor, target) == 0:
            return steps
        if not cursor.goRight(1, False):
            break
    raise Refused('unrepresentable_range')


class ReplacementUndo(unohelper.Base, XUndoAction, XComponent):
    active = 0
    """One qualified range replacement, never whole-document restore."""
    def __init__(self, document, selected, before, after, start, before_steps, before_hash, after_hash, formatting=None, mixed_format=False, before_runs=None):
        self.document = document
        self.formatting = formatting
        self.before_runs = before_runs
        self.mixed_format = mixed_format
        self.before, self.after = before, after
        self.start = start
        self.before_steps = before_steps
        self.after_steps = movement_to(document, selected.End) - start
        self.before_hash, self.after_hash = before_hash, after_hash
        self.Title = 'Grammar reviewed replacement'
        self.disposed = False
        self.listeners = []
        ReplacementUndo.active += 1

    def dispose(self):
        if self.disposed:
            return
        self.disposed = True
        self.document = None
        self.formatting = None
        self.before_runs = None
        self.before = self.after = None
        self.before_hash = self.after_hash = None
        ReplacementUndo.active -= 1
        listeners, self.listeners = self.listeners, []
        event = uno.createUnoStruct('com.sun.star.lang.EventObject')
        event.Source = self
        for listener in listeners:
            try:
                listener.disposing(event)
            except Exception:
                pass

    def addEventListener(self, listener):
        if self.disposed:
            event = uno.createUnoStruct('com.sun.star.lang.EventObject')
            event.Source = self
            listener.disposing(event)
        else:
            self.listeners.append(listener)

    def removeEventListener(self, listener):
        if listener in self.listeners:
            self.listeners.remove(listener)

    def replace(self, expected, replacement, steps, digest):
        if self.disposed:
            raise UndoFailedException('Disposed Grammar action', self, None)
        doc = self.document
        if document_fingerprint(doc, self.formatting is not None, self.mixed_format) != digest or not doc.isSetModifiedEnabled():
            raise UndoFailedException('Grammar document changed; no replacement', self, None)
        selected = doc.Text.createTextCursor()
        selected.gotoStart(False)
        if not selected.goRight(self.start, False) or not selected.goRight(steps, True) or selected.String != expected:
            raise UndoFailedException('Grammar range changed; no replacement', self, None)
        manager = doc.getUndoManager()
        # Native Undo/Redo owns the recording lock while invoking XUndoAction.
        if not manager.isLocked():
            raise UndoFailedException('Undo manager must own recording lock', self, None)
        doc.disableSetModified()
        try:
            if selected.String != expected:
                raise UndoFailedException('Grammar range changed', self, None)
            selected.String = replacement
            if self.formatting is not None:
                if self.before_runs is not None and digest == self.after_hash:
                    restore_selected_runs(doc, selected, self.before_runs, self.formatting)
                else:
                    restore_uniform_format(selected, self.formatting)
        finally:
            doc.enableSetModified()
            doc.setModified(True)
        if self.mixed_format:
            expected_hash = self.before_hash if digest == self.after_hash else self.after_hash
            if document_fingerprint(doc, True, self.mixed_format) != expected_hash:
                raise UndoFailedException('Postwrite result changed', self, None)
        elif selected.String != replacement or (self.formatting is not None
                and plain_metadata(doc, True) != self.formatting):
            raise UndoFailedException('Postwrite result changed', self, None)

    def undo(self):
        self.replace(self.after, self.before, self.after_steps, self.after_hash)

    def redo(self):
        self.replace(self.before, self.after, self.before_steps, self.before_hash)


class Changes(unohelper.Base, XModifyListener, XSelectionChangeListener, XDocumentEventListener):
    def __init__(self, owner):
        self.owner = owner

    def modified(self, event):
        if self.owner:
            self.owner.revision += 1

    def selectionChanged(self, event):
        if self.owner:
            self.owner.selection_revision += 1

    def documentEventOccured(self, event):
        # Do not inspect event supplements or other documents. Save/rename/reload
        # invalidates authority even when the URL and text later return to A.
        owner = self.owner
        if owner and event.Source == owner.document and event.EventName in (
                'OnSave', 'OnSaveDone', 'OnSaveFailed', 'OnSaveAs', 'OnSaveAsDone',
                'OnSaveAsFailed', 'OnSaveTo', 'OnSaveToDone', 'OnSaveToFailed',
                'OnPrepareUnload', 'OnUnload', 'OnLoadFinished', 'OnModeChanged'):
            owner.lifecycle_events += 1
            owner.epoch += 1
            owner.revoked = True
            owner.clear_review()

    def disposing(self, event):
        if self.owner:
            self.owner.clear_review()
            self.owner.document_events = False
            self.owner.location = None
            self.owner.document = self.owner.controller = self.owner.listener = None
            self.owner = None


class Expiry(unohelper.Base, XCallback):
    def __init__(self, owner, token):
        self.owner, self.token = owner, token

    def notify(self, ignored):
        owner = self.owner
        if owner.review and owner.review['token'] == self.token and time.monotonic() >= owner.review['expires']:
            owner.clear_review()
            if owner.result and owner.result.get('token') == self.token:
                owner.result = dict(status='EXPIRED')


class Callback(unohelper.Base, XCallback):
    def __init__(self, owner, args, deadline, epoch):
        self.owner, self.args, self.deadline, self.epoch = owner, args, deadline, epoch

    def notify(self, ignored):
        owner = self.owner
        wrote = False
        try:
            owner.deadline(self.deadline)
            action = self.args['Action']
            if action != 'disable' and owner.epoch != self.epoch:
                raise Refused('permission_revoked')
            if action == 'enable':
                owner.disable()
                document = self.args['Document']
                owner.check_document(document)
                owner.document = document
                owner.location = document.getURL()
                owner.controller = document.CurrentController
                owner.window = int(owner.controller.Frame.ContainerWindow.getWindowHandle((), SYSTEM_WIN32))
                owner.listener = Changes(owner)
                document.addModifyListener(owner.listener)
                owner.controller.addSelectionChangeListener(owner.listener)
                if owner.allow_saved_documents:
                    document.addDocumentEventListener(owner.listener)
                    owner.document_events = True
                    owner.check_document(document)
                if owner.epoch != self.epoch:
                    raise Refused('permission_revoked')
                owner.revoked = False
                value = dict(status='ENABLED')
            elif action == 'disable':
                owner.disable()
                value = dict(status='DISABLED')
            elif action == 'dismiss':
                owner.clear_review()
                value = dict(status='DISMISSED')
            elif action == 'capture':
                owner.clear_review()
                owner.permission(self.epoch)
                owner.check_document(owner.document)
                owner.foreground_document()
                selected = owner.selected()
                source = owner.read_range(selected)
                if not source or len(source) > 4096:
                    raise Refused('source_bounds')
                if owner.allow_uniform_format and not owner.allow_span and any(c in source for c in '\r\n\u2028\u2029'):
                    raise Refused('formatted_single_paragraph_only')
                owner.deadline(self.deadline)
                owner.permission(self.epoch)
                owner.foreground_document()
                layout = plain_metadata(owner.document, True, owner.allow_mixed_format) if owner.allow_mixed_format else None
                span = owner.allow_span and any(c in source for c in '\r\n')
                if span:
                    prefix=owner.document.Text.createTextCursor()
                    prefix.gotoStart(False);prefix.gotoRange(selected.Start,True)
                    paragraph_span_plan(owner.document.Text.String,layout,len(prefix.String),len(source),source)
                mixed_range = mixed_selection(owner.document, selected, layout, owner.allow_cross_format) if layout is not None and not span else None
                owner.deadline(self.deadline)
                owner.permission(self.epoch)
                owner.review = dict(token=secrets.token_hex(24), source=source,
                    range=owner.document.Text.createTextCursorByRange(selected),
                    revision=owner.revision, selection_revision=owner.selection_revision,
                    expires=time.monotonic() + 30, epoch=self.epoch,
                    layout=layout, mixed_range=mixed_range, span=span,
                    formatting=mixed_range[2] if mixed_range else
                        (plain_metadata(owner.document, True) if owner.allow_uniform_format and not span else None))
                owner.schedule_expiry(owner.review['token'])
                value = dict(status='CAPTURED', token=owner.review['token'], source=source)
            elif action == 'accept':
                review = owner.review
                # Consume authority before any callback-producing method or attempted write.
                owner.clear_review()
                if not review or not secrets.compare_digest(str(self.args.get('Token', '')), review['token']):
                    raise Refused('token')
                replacement = self.args.get('Replacement')
                if not isinstance(replacement, str) or not replacement or len(replacement) > 4096:
                    raise Refused('replacement_bounds')
                if review['formatting'] is not None and any(c in replacement for c in '\r\n\u2028\u2029'):
                    raise Refused('formatted_single_paragraph_only')
                if review.get('span'):
                    value,wrote=apply_paragraph_span(owner,review,replacement,self.deadline)
                else:
                    owner.validate(review, self.deadline)
                    undo = owner.document.getUndoManager()
                    undo.enterUndoContext('Grammar experimental reviewed replacement')
                    try:
                        # enterUndoContext can call listeners. Recheck everything after it.
                        owner.validate(review, self.deadline)
                        # Record one custom action after replacement, while native
                        # sub-operation notifications/recording are suppressed. Existing
                        # history is retained; no clear/reset or whole-document rollback.
                        if undo.isLocked():
                            raise Refused('undo_owned_elsewhere')
                        # The bounded local body hash qualifies custom Undo/Redo
                        # after earlier native history rebuilds live range objects.
                        before_hash = document_fingerprint(owner.document, owner.allow_uniform_format, owner.allow_mixed_format)
                        expected_after_hash = None
                        if review['layout'] is not None:
                            offset, length, _ = review['mixed_range']
                            expected_layout = replaced_layout(review['layout'], offset, length, len(replacement), owner.allow_cross_format)
                            body = body_snapshot(owner.document, True, owner.allow_mixed_format)
                            expected_body = body[:offset] + replacement + body[offset + length:]
                            expected_after_hash = fingerprint(json.dumps((expected_body, expected_layout),
                                ensure_ascii=True, separators=(',', ':')))
                        start = movement_to(owner.document, review['range'].Start)
                        before_steps = movement_to(owner.document, review['range'].End) - start
                        undo.lock()
                        try:
                            owner.validate(review, self.deadline)
                            owner.document.disableSetModified()
                            try:
                                owner.validate(review, self.deadline, own_notification_batch=True)
                                wrote = True
                                review['range'].String = replacement
                                if review['formatting'] is not None:
                                    restore_uniform_format(review['range'], review['formatting'])
                            finally:
                                owner.document.enableSetModified()
                        finally:
                            undo.unlock()
                        after_hash = document_fingerprint(owner.document, owner.allow_uniform_format, owner.allow_mixed_format)
                        if expected_after_hash is not None and after_hash != expected_after_hash:
                            raise Refused('postwrite_layout_mismatch')
                        undo.addUndoAction(ReplacementUndo(owner.document, review['range'], review['source'], replacement,
                            start, before_steps, before_hash, after_hash, review['formatting'], owner.allow_mixed_format,
                            selected_layout_runs(review['layout'], *review['mixed_range'][:2]) if owner.allow_cross_format else None))
                        owner.document.setModified(True)
                    finally:
                        undo.leaveUndoContext()
                    if (owner.read_range(review['range']) != replacement or (owner.allow_mixed_format
                            and document_fingerprint(owner.document, True, owner.allow_mixed_format) != expected_after_hash)
                            or (not owner.allow_mixed_format and review['formatting'] is not None
                            and plain_metadata(owner.document, True) != review['formatting'])):
                        value = dict(status='UNKNOWN_CHANGED', reason='postwrite_mismatch')
                    else:
                        value = dict(status='APPLIED', undo=bool(undo.isUndoPossible()))
            else:
                raise Refused('action')
        except Refused as error:
            value = dict(status='UNKNOWN_CHANGED' if wrote else 'REJECTED', reason=str(error))
        except Exception as error:
            value = dict(status='UNKNOWN_CHANGED' if wrote else 'ERROR', reason=type(error).__name__ + ': ' + str(error))
        finally:
            self.args = None
            if owner.revoked:
                try:
                    owner.disable()
                    if wrote:
                        value = dict(status='UNKNOWN_CHANGED', reason='revoked_after_write')
                except Exception as error:
                    value = dict(status='UNKNOWN_CHANGED' if wrote else 'ERROR', reason='detach: ' + str(error))
        owner.result = value
        owner.pending = False


class WriterCore(unohelper.Base, XJob):
    def __init__(self, context, *, allow_saved_documents=False, allow_uniform_format=False, allow_mixed_format=False, allow_paragraphs=False, allow_cross_format=False, allow_span=False):
        self.context = context
        self.desktop = context.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', context)
        self.document = self.controller = self.listener = self.review = self.result = None
        self.revision = self.selection_revision = 0
        self.pending = False
        self.expiry_timer = None
        self.text_reads = 0
        self.epoch = 0
        self.revoked = True
        self.window = 0
        # Deployment capabilities are explicit and default off.
        self.allow_saved_documents = allow_saved_documents
        if allow_cross_format and not allow_paragraphs:
            raise ValueError('cross format requires paragraph mode')
        if allow_span and not allow_cross_format:
            raise ValueError('span requires cross formatting')
        self.allow_span = allow_span
        self.allow_cross_format = allow_cross_format
        self.allow_uniform_format = allow_uniform_format
        if allow_mixed_format and not allow_uniform_format:
            raise ValueError('mixed format requires formatted mode')
        self.allow_mixed_format = allow_mixed_format
        if allow_paragraphs:
            if not allow_mixed_format:
                raise ValueError('paragraph mode requires mixed formatting')
            self.allow_mixed_format = 'paragraphs'
        self.location = None
        self.document_events = False
        self.lifecycle_events = 0

    def deadline(self, expiry):
        ticks = None
        if isinstance(expiry, tuple):
            expiry, ticks = expiry
        if ticks is not None:
            tick_clock = ctypes.windll.kernel32.GetTickCount64
            tick_clock.restype = ctypes.c_ulonglong
            if tick_clock() >= ticks:
                raise Refused('expired_operation')
        if time.monotonic() >= expiry:
            raise Refused('expired_operation')

    def clear_review(self):
        self.review = None
        if self.expiry_timer:
            self.expiry_timer.cancel()
            self.expiry_timer = None

    def schedule_expiry(self, token):
        def post():
            callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
            callback.addCallback(Expiry(self, token), None)
        self.expiry_timer = threading.Timer(30, post)
        self.expiry_timer.daemon = True
        self.expiry_timer.start()

    def disable(self):
        self.revoked = True
        self.clear_review()
        if self.listener:
            if self.document_events:
                self.document.removeDocumentEventListener(self.listener)
                self.document_events = False
            try:
                self.document.removeModifyListener(self.listener)
                self.controller.removeSelectionChangeListener(self.listener)
            finally:
                self.listener.owner = None
                self.listener = None
        self.document = self.controller = None
        self.location = None

    def check_document(self, doc, own_notification_batch=False):
        if doc is None:
            raise Refused('disabled')
        # An explicit paired deployment scopes admission; never trust file suffix alone.
        if not doc.supportsService('com.sun.star.text.TextDocument') or doc.isReadonly():
            raise Refused('unsupported_document')
        location = doc.getURL()
        if doc.hasLocation():
            if not self.allow_saved_documents:
                raise Refused('unsupported_document')
            # Local DOCX only; no network documents or implicit filesystem writes.
            if not location.startswith('file:///') or not location.lower().endswith('.docx'):
                raise Refused('unsupported_location')
            drive, tail = ntpath.splitdrive(uno.fileUrlToSystemPath(location))
            if len(drive) != 2 or drive[1] != ':' or not tail.startswith('\\'):
                raise Refused('unsupported_location')
            if any(arg.Name in ('Password', 'EncryptionData', 'ModifyPasswordInfo') for arg in doc.getArgs()):
                raise Refused('protected_document')
            filters = [arg.Value for arg in doc.getArgs() if arg.Name == 'FilterName']
            if filters != ['Office Open XML Text']:
                raise Refused('unsupported_filter')
        if self.location is not None and location != self.location:
            raise Refused('document_location_changed')
        if not own_notification_batch and not doc.isSetModifiedEnabled():
            raise Refused('modification_state_owned_elsewhere')
        plain_metadata(doc, self.allow_uniform_format, self.allow_mixed_format)
        if self.desktop.CurrentComponent != doc:
            raise Refused('document_changed')
        controller = doc.CurrentController
        if self.controller is not None and controller != self.controller:
            raise Refused('controller_changed')
        hwnd = int(controller.Frame.ContainerWindow.getWindowHandle((), SYSTEM_WIN32))
        pid = ctypes.c_ulong()
        api = ctypes.windll.user32.GetWindowThreadProcessId
        api.argtypes = (ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong))
        api.restype = ctypes.c_ulong
        tid = api(ctypes.c_void_p(hwnd), ctypes.byref(pid))
        if pid.value != os.getpid() or tid != threading.get_native_id():
            raise Refused('wrong_thread')

    def selected(self):
        selection = self.controller.getSelection()
        if selection.getCount() != 1:
            raise Refused('multiple_ranges')
        selected = selection.getByIndex(0)
        if selected.Text != self.document.Text:
            raise Refused('non_body_range')
        return selected

    def foreground_document(self):
        foreground = ctypes.windll.user32.GetForegroundWindow
        foreground.restype = ctypes.c_void_p
        if foreground() != self.window:
            raise Refused('document_not_foreground')

    def read_range(self, selected):
        bounded = self.document.Text.createTextCursorByRange(selected.Start)
        bounded.goRight(4096, True)
        if self.document.Text.compareRegionEnds(bounded, selected) > 0:
            raise Refused('source_bounds')
        self.text_reads += 1
        return selected.String

    def validate(self, review, deadline, own_notification_batch=False):
        self.permission(review['epoch'])
        self.deadline(deadline)
        if time.monotonic() >= review['expires']:
            raise Refused('expired_review')
        self.check_document(self.document, own_notification_batch)
        if review.get('layout') is not None and plain_metadata(self.document, True, self.allow_mixed_format) != review['layout']:
            raise Refused('format_changed')
        if not self.allow_mixed_format and review['formatting'] is not None and plain_metadata(self.document, True) != review['formatting']:
            raise Refused('format_changed')
        if self.revision != review['revision'] or self.selection_revision != review['selection_revision']:
            raise Refused('revision_changed')
        selected = self.selected()
        text = self.document.Text
        if text.compareRegionStarts(selected, review['range']) or text.compareRegionEnds(selected, review['range']):
            raise Refused('range_changed')
        if self.read_range(selected) != review['source'] or self.read_range(review['range']) != review['source']:
            raise Refused('source_changed')
        self.deadline(deadline)
        self.permission(review['epoch'])

    def permission(self, epoch):
        if self.revoked or self.epoch != epoch:
            raise Refused('permission_revoked')

    def execute(self, arguments):
        args = {arg.Name: arg.Value for arg in arguments}
        if args.get('Action') == 'state':
            return json.dumps(dict(status='STATE', reviewRetained=self.review is not None,
                resultRetained=self.result is not None, enabled=self.document is not None and not self.revoked,
                bindingRetained=self.document is not None, revoked=self.revoked,
                lifecycleEvents=self.lifecycle_events, documentEventsAttached=self.document_events,
                pending=self.pending, textReads=self.text_reads, undoActions=ReplacementUndo.active))
        if args.get('Action') == 'poll':
            value = self.result
            self.result = None
            return json.dumps(value or dict(status='PENDING'))
        if args.get('Action') == 'disable':
            # Revoke immediately even during an in-flight callback; detachment belongs
            # to its UI-thread finally block. REVOKING is not a cleanup-completed claim.
            self.epoch += 1
            self.revoked = True
            if self.pending:
                return json.dumps(dict(status='REVOKING'))
            self.result = None
        if self.pending or self.result is not None:
            return json.dumps(dict(status='BUSY'))
        expiry = args.get('Deadline')
        if not isinstance(expiry, (float, int)) or not math.isfinite(expiry) or not 0 < expiry - time.monotonic() <= 3:
            return json.dumps(dict(status='REJECTED', reason='deadline'))
        ticks = args.get('DeadlineTicks')
        if ticks is not None:
            tick_clock = ctypes.windll.kernel32.GetTickCount64
            tick_clock.restype = ctypes.c_ulonglong
            if isinstance(ticks, bool) or not isinstance(ticks, int) or not 0 < ticks - tick_clock() <= 3000:
                return json.dumps(dict(status='REJECTED', reason='deadline_ticks'))
            expiry = (expiry, ticks)
        self.pending = True
        try:
            callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
            callback.addCallback(Callback(self, args, expiry, self.epoch), None)
        except Exception:
            self.pending = False
            raise
        return json.dumps(dict(status='QUEUED'))

