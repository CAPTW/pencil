"""Deterministic production layout transform tests; not native qualification."""
import ast
from pathlib import Path
import unittest

source = Path(__file__).resolve().parents[2] / 'adapters/libreoffice/pythonpath/grammar_writer_core.py'
tree = ast.parse(source.read_text(encoding='utf-8'))
namespace = {}
selected = [node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef))
            and node.name in ('Refused', 'replaced_layout')]
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
