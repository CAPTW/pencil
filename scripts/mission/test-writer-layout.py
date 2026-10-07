"""Deterministic production layout transform tests; not native qualification."""
import ast
from pathlib import Path
import unittest

source = Path(__file__).resolve().parents[2] / 'adapters/libreoffice/pythonpath/grammar_writer_core.py'
tree = ast.parse(source.read_text(encoding='utf-8'))
namespace = {'FORMATTED_CHARACTERS': ('CharColor',)}
selected = [node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef))
            and node.name in ('Refused', 'selected_layout_runs', 'replaced_layout', 'restore_uniform_format', 'paragraph_span_plan', 'effective_character_format', 'uniform_topology_plan')]
exec(compile(ast.Module(body=selected, type_ignores=[]), str(source), 'exec'), namespace)
replace = namespace['replaced_layout']
Refused = namespace['Refused']


class Tests(unittest.TestCase):
    def test_all_homogeneous_ranges_preserve_unselected_characters_and_reverse(self):
        # Independent per-character oracle: every position and length, including
        # exact run endpoints, short/long replacement, and adjacent same colors.
        runs = ((3, ('blue',), ()), (5, ('red',), ()), (2, ('blue',), ()))
        layout = (runs, ('center',), ('defaults',))
        chars = [style for size, style, _ in runs for _ in range(size)]
        for start in range(len(chars)):
            for size in range(1, len(chars) - start + 1):
                for new_size in (1, 4, 12):
                    with self.subTest(start=start, size=size, new_size=new_size):
                        if len(set(chars[start:start + size])) != 1:
                            with self.assertRaises(Refused):
                                replace(layout, start, size, new_size)
                            continue
                        changed = replace(layout, start, size, new_size)
                        expected = chars[:start] + [chars[start]] * new_size + chars[start + size:]
                        actual = [style for count, style, _ in changed[0] for _ in range(count)]
                        self.assertEqual(actual, expected)
                        self.assertEqual(changed[1:], layout[1:])
                        self.assertEqual(replace(changed, start, new_size, size), layout)

    def test_cross_format_first_style_and_original_run_capture(self):
        runs = ((3, ('blue',), ()), (5, ('red',), ()), (2, ('green',), ()))
        layout = (runs, ('center',), ())
        chars = [style for size, style, _ in runs for _ in range(size)]
        for start in range(10):
            for size in range(1, 11-start):
                original = namespace['selected_layout_runs'](layout, start, size)
                self.assertEqual([c for n,c,_ in original for _ in range(n)], chars[start:start+size])
                for count in (1, 4, 12):
                    actual = replace(layout, start, size, count, True)
                    self.assertEqual([c for n,c,_ in actual[0] for _ in range(n)],
                        chars[:start] + [chars[start]]*count + chars[start+size:])

    def test_inherited_effective_color_is_restored_not_reset(self):
        class Cursor:
            CharInteropGrabBag = ()
            value = 0x118822
            def getPropertyValue(self, name): return self.value
            def setPropertyValue(self, name, value): self.value = value
            def setPropertyToDefault(self, name): self.value = -1
        cursor = Cursor()
        namespace['restore_uniform_format'](cursor, ((('CharColor', 0xBB2200, 'DEFAULT_VALUE'),), (), (), ()))
        self.assertEqual(cursor.value, 0xBB2200)

    def test_span_plan_preserves_separators_empty_style_and_alignments(self):
        body = 'abcd\r\n\r\nxyz'
        layout = (((4, ('red',), ()),(2, (), ('paragraph_break','\r\n')),
            (0, ('empty',),()),(2,(),('paragraph_break','\r\n')),(3,('blue',),())),
            ('paragraphs',((0,'left'),(6,'center'),(8,'right'))),())
        plan, text, changed = namespace['paragraph_span_plan'](body, layout, 2, 8, 'LONG\n\nQ')
        self.assertEqual(text, 'abLONG\r\n\r\nQz')
        self.assertEqual(changed[1], ('paragraphs',((0,'left'),(8,'center'),(10,'right'))))
        self.assertIn((0,('empty',),()),changed[0])
        self.assertEqual([(a,n,v) for a,n,v,_,_ in plan],[(8,2,'Q'),(2,2,'LONG')])
        for replacement in ('one','a\nb','a\nx\nb','a\n\n','a\u2028b'):
            with self.assertRaises(Refused):
                namespace['paragraph_span_plan'](body,layout,2,8,replacement)
        with self.assertRaises(Refused):
            namespace['paragraph_span_plan'](body,layout,5,5,'x\ny')

    def test_span_plan_all_partial_endpoints_independent_oracle(self):
        body='abcde\r\nfghij\r\nklmn'
        layout=(((5,('red',),()),(2,(),('paragraph_break','\r\n')),
            (5,('green',),()),(2,(),('paragraph_break','\r\n')),(4,('blue',),())),
            ('paragraphs',((0,'left'),(7,'center'),(14,'right'))),())
        for start in range(5):
            for end in range(1,5):
                for count in (1,3,8):
                    values=['X'*count,'Y'*(count+1),'Z'*(count+2)]
                    _,text,result=namespace['paragraph_span_plan'](body,layout,start,14+end-start,'\n'.join(values))
                    expected=[body[:start]+values[0],values[1],values[2]+body[14+end:]]
                    self.assertEqual(text,'\r\n'.join(expected))
                    self.assertEqual(result[1],('paragraphs',((0,'left'),(len(expected[0])+2,'center'),
                        (len(expected[0])+len(expected[1])+4,'right'))))
                    actual=[style for n,style,_ in result[0] for _ in range(n)]
                    oracle=[('red',)]*len(expected[0])+[()]*2+[('green',)]*len(expected[1])+[()]*2+[('blue',)]*len(expected[2])
                    self.assertEqual(actual,oracle)

    def test_direct_paragraph_character_property_is_not_style_inheritance(self):
        from types import SimpleNamespace
        class Props:
            def __init__(self,value,state):self.value,self.state=value,state
            def getPropertyValue(self,name):return self.value
            def getPropertyState(self,name):return SimpleNamespace(value=self.state)
        normal=namespace['effective_character_format']
        local=Props(17,'DEFAULT_VALUE')
        self.assertEqual(normal(local,Props(17,'DIRECT_VALUE')),(('CharColor',17,'DIRECT_VALUE'),))
        self.assertEqual(normal(local,Props(17,'DEFAULT_VALUE')),(('CharColor',17,'DEFAULT_VALUE'),))
        self.assertEqual(normal(local,Props(18,'DIRECT_VALUE')),(('CharColor',17,'DEFAULT_VALUE'),))

    def test_uniform_topology_merge_split_and_empty_parts(self):
        plan = namespace['uniform_topology_plan']
        body = 'abc\r\n\r\ndef'
        layout = (((3,('red',),()),(2,(),('paragraph_break','\r\n')),
            (0,('red',),()),(2,(),('paragraph_break','\r\n')),(3,('red',),())),
            ('paragraphs',((0,'left'),(5,'left'),(7,'left'))),())
        for value in ('joined','a\n\nb','\n','x\ny\nz\nw','한국어 😀'):
            text,result,fmt = plan(body,layout,1,8,value)
            expected = 'a'+value.replace('\n','\r\n')+'f'
            self.assertEqual(text,expected)
            parts=expected.split('\r\n')
            offsets=[]; offset=0
            for part in parts:
                offsets.append((offset,'left'));offset+=len(part)+2
            self.assertEqual(result[1],('paragraphs',tuple(offsets)))
            self.assertEqual(sum(n for n,_,_ in result[0]),len(text))
            self.assertEqual(fmt,(('red',),'left',(),()))
        different=(layout[0],('paragraphs',((0,'left'),(5,'center'),(7,'left'))),())
        with self.assertRaises(Refused):plan(body,different,1,8,'merged')
        for value in ('a\rb','a\u2028b','\n'*8,''):
            with self.assertRaises(Refused):plan(body,layout,1,8,value)
        with self.assertRaises(Refused):plan(body,layout,4,3,'x')

    def test_topology_outside_styles_and_all_partial_endpoints(self):
        plan=namespace['uniform_topology_plan']
        parts=['OUT','abc','de','TAIL']
        styles=[('blue',),('red',),('red',),('green',)]
        alignments=['left','center','center','right']
        runs=[];positions=[];offset=0
        for i,part in enumerate(parts):
            positions.append((offset,alignments[i]))
            runs.append((len(part),styles[i],()))
            offset+=len(part)
            if i<3:runs.append((2,(),('paragraph_break','\r\n')));offset+=2
        layout=(tuple(runs),('paragraphs',tuple(positions)),('defaults',))
        body='\r\n'.join(parts)
        for a in range(4):
            for b in range(3):
                for replacement in ('X','X\nY','\n','X\n\nY','😀\n한국어'):
                    start=5+a;end=10+b
                    actual,result,_=plan(body,layout,start,end-start,replacement)
                    changed=(parts[1][:a]+replacement+parts[2][b:]).split('\n')
                    expected=['OUT']+changed+['TAIL']
                    self.assertEqual(actual,'\r\n'.join(expected))
                    char_oracle=[('blue',)]*3+[()]*2
                    for i,part in enumerate(changed):
                        char_oracle += [('red',)]*len(part)+[()]*2
                    char_oracle += [('green',)]*4
                    self.assertEqual([style for n,style,_ in result[0] for _ in range(n)],char_oracle)
                    offset=0;expected_positions=[]
                    for i,part in enumerate(expected):
                        expected_positions.append((offset,'left' if i==0 else 'right' if i==len(expected)-1 else 'center'))
                        offset+=len(part)+2
                    self.assertEqual(result[1][1],tuple(expected_positions))

    def test_topology_never_converts_existing_soft_breaks_to_paragraphs(self):
        for separator in ('\n','\r','\u2028','\u2029'):
            body='a'+separator+'b'
            layout=(((3,('red',),()),),('paragraphs',((0,'left'),)),())
            with self.assertRaises(Refused):
                namespace['uniform_topology_plan'](body,layout,0,1,'replacement')

    def test_invalid_ranges_fail_closed(self):
        layout = (((4, ('blue',), ()),), (), ())
        for start, size, new_size in [(-1, 2, 1), (5, 1, 1), (3, 2, 1), (0, 0, 1), (0, 1, 0)]:
            with self.subTest(start=start, size=size, new_size=new_size):
                with self.assertRaises(Refused):
                    replace(layout, start, size, new_size)

    def test_paragraph_positions_and_empty_paragraph_are_preserved(self):
        layout = (((3, ('red',), ()), (1, (), ('paragraph_break',)), (0, ('empty-style',), ()),
                   (1, (), ('paragraph_break',)), (4, ('blue',), ())),
                  ('paragraphs', ((0, 'left'), (4, 'center'), (5, 'right'))), ('defaults',))
        changed = replace(layout, 1, 2, 5)
        self.assertEqual(changed[1], ('paragraphs', ((0, 'left'), (7, 'center'), (8, 'right'))))
        self.assertEqual(replace(changed, 1, 5, 2), layout)
        changed = replace(layout, 5, 4, 2)
        self.assertEqual(changed[1], layout[1])
        with self.assertRaises(Refused):
            replace(layout, 2, 4, 1)

    def test_crlf_width_is_distinct_from_native_cursor_steps(self):
        layout = (((3, ('red',), ()), (2, (), ('paragraph_break', '\r\n')), (4, ('blue',), ())),
            ('paragraphs', ((0, 'left'), (5, 'right'))), ())
        changed = replace(layout, 0, 3, 1)
        self.assertEqual(changed[1][1], ((0, 'left'), (3, 'right')))
        self.assertEqual(replace(changed, 0, 1, 3), layout)
        with self.assertRaises(Refused): replace(layout, 2, 2, 1)


if __name__ == '__main__':
    unittest.main()
