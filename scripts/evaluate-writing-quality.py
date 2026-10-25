"""Offline aggregation of explicit human reviews; no inference or semantic claims."""
import argparse
import json
import math
from pathlib import Path

AXES = ('meaning', 'roles', 'logic', 'register', 'entities', 'naturalness')
MODELS = ('gpt-6-luna', 'claude-haiku-5-5', 'gemma4-12b-qat-q4_0')

def evaluate(cases, rows):
    expected = {case['id'] for case in cases}
    seen = set()
    for row in rows:
        identity = (row['model'], row['case'])
        if identity in seen or row['model'] not in MODELS or row['case'] not in expected:
            raise ValueError('duplicate or unknown model/case')
        seen.add(identity)
        if row['status'] not in ('accepted', 'rejected', 'failed', 'unavailable'):
            raise ValueError('invalid status')
        if row['status'] == 'accepted':
            if not row.get('output', '').strip() or not row.get('reviewer'):
                raise ValueError('accepted output needs explicit review')
            scores = row.get('scores', {})
            if set(scores) != set(AXES) or any(type(x) is not int or not 0 <= x <= 4 for x in scores.values()):
                raise ValueError('all six review axes required')
            if type(row.get('criticalError')) is not bool:
                raise ValueError('critical error judgment required')
            latency = row.get('recommendationMs')
            if type(latency) not in (int, float) or not math.isfinite(latency) or latency < 0:
                raise ValueError('finite end-to-end latency required')
    reports = {}
    for model in MODELS:
        group = [row for row in rows if row['model'] == model]
        accepted = [row for row in group if row['status'] == 'accepted']
        passed = [row for row in accepted if not row['criticalError'] and min(row['scores'].values()) >= 3]
        timings = sorted(row['recommendationMs'] for row in accepted)
        reports[model] = {
            'coverage': len(group), 'expected': len(expected),
            'missing': sorted(expected - {row['case'] for row in group}),
            'qualityPassed': len(passed), 'qualityDenominator': len(expected),
            'failuresOrRefusals': len(group) - len(accepted),
            'criticalErrors': sum(row['criticalError'] for row in accepted),
            'acceptedLatencyOnly': {name: timings[max(0, math.ceil(len(timings)*q)-1)] if timings else None
                                    for name, q in [('p50', .5), ('p95', .95), ('max', 1)]},
            'qualification': 'REVIEWED_CORPUS_ONLY' if len(passed) == len(expected) else 'INCOMPLETE_OR_FAILED'
        }
    return reports

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('receipts', type=Path, help='JSON array of synthetic, human-reviewed results')
    args = parser.parse_args()
    cases = json.loads((Path(__file__).resolve().parents[1]/'tests/quality/writing-cases.json').read_text(encoding='utf-8'))
    print(json.dumps(evaluate(cases, json.loads(args.receipts.read_text(encoding='utf-8'))), indent=2))
