import importlib.util
from pathlib import Path
import unittest
spec = importlib.util.spec_from_file_location('quality', Path(__file__).with_name('evaluate-writing-quality.py'))
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)

class EvaluationTests(unittest.TestCase):
    cases = [{'id': 'one'}, {'id': 'two'}]
    def row(self):
        return dict(model='gpt-6-luna', case='one', status='accepted', output='Synthetic.',
                    reviewer='blind-reviewer-A', scores={axis: 4 for axis in quality.AXES},
                    criticalError=False, recommendationMs=1200)
    def test_missing_models_and_cases_never_pass(self):
        result = quality.evaluate(self.cases, [self.row()])
        self.assertEqual(result['gpt-6-luna']['qualification'], 'INCOMPLETE_OR_FAILED')
        self.assertEqual(result['claude-haiku-5-5']['coverage'], 0)
    def test_critical_error_overrides_fluent_scores(self):
        row = self.row(); row['criticalError'] = True
        self.assertEqual(quality.evaluate(self.cases, [row])['gpt-6-luna']['qualityPassed'], 0)
    def test_refusal_counts_in_denominator(self):
        rows = [self.row(), dict(model='gpt-6-luna', case='two', status='rejected')]
        report = quality.evaluate(self.cases, rows)['gpt-6-luna']
        self.assertEqual((report['qualityPassed'], report['qualityDenominator']), (1, 2))
    def test_duplicates_and_unreviewed_outputs_refused(self):
        with self.assertRaises(ValueError): quality.evaluate(self.cases, [self.row(), self.row()])
        row = self.row(); del row['reviewer']
        with self.assertRaises(ValueError): quality.evaluate(self.cases, [row])

if __name__ == '__main__': unittest.main()
