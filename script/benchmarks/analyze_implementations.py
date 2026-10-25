"""Summarize fresh-process timings with paired bootstrap intervals by round."""
import argparse
import json
from pathlib import Path
import random
import statistics

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('results', type=Path)
args = parser.parse_args()
report = json.loads(args.results.read_text())
rng = random.Random(report['metadata']['seed'])


def compare(left, right):
    assert len(left) == len(right) and len(left) >= 3
    differences = []
    for _ in range(10000):
        indexes = [rng.randrange(len(left)) for _ in left]
        differences.append(statistics.median(left[i] for i in indexes)
                           - statistics.median(right[i] for i in indexes))
    differences.sort()
    return {'median_difference_ms': statistics.median(left) - statistics.median(right),
            'ratio_of_medians': statistics.median(left) / statistics.median(right),
            'difference_95_percent_interval_ms': [differences[250], differences[9749]]}


analysis = {'source_results': args.results.name, 'bootstrap_samples': 10000,
            'estimator': 'difference of medians; paired bootstrap by randomized round',
            'seed': report['metadata']['seed'], 'workloads': []}
print('| Workload | Python ms | Rust ms | Swift ms |')
print('| --- | ---: | ---: | ---: |')
for item in report['workloads']:
    modes = item['implementations']
    row = {'name': item['name'], 'comparisons': {}}
    for left, right in [('python', 'rust'), ('python', 'swift'), ('swift', 'rust')]:
        row['comparisons'][left + '_minus_' + right] = compare(modes[left]['samples_ms'], modes[right]['samples_ms'])
    analysis['workloads'].append(row)
    print('| ' + item['name'] + ' | ' + ' | '.join(f"{modes[name]['median_ms']:.2f}" for name in ('python', 'rust', 'swift')) + ' |')
output = args.results.with_name('analysis.json')
output.write_text(json.dumps(analysis, indent=2) + '\n')
print('Analysis: ' + str(output))
