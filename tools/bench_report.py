#!/usr/bin/env python3
import json
from pathlib import Path

root = Path('target/criterion')
entries = []
for d in sorted(root.iterdir()):
    if not d.is_dir():
        continue
    est = d / 'new' / 'estimates.json'
    if not est.exists():
        continue
    data = json.loads(est.read_text())
    entries.append((d.name, data))

report = []
report.append('# QRD Benchmark Report\n')
report.append('Generated from Criterion outputs in `target/criterion`\n')
report.append('\n')
report.append('| Benchmark | Mean (ns) | Median (ns) | Std Dev (ns) | 95% CI Mean Lower | 95% CI Mean Upper |')
report.append('|---|---:|---:|---:|---:|---:|')
for name, data in entries:
    mean = data.get('mean', {})
    median = data.get('median', {})
    std = data.get('std_dev', {})
    mean_val = mean.get('point_estimate')
    mean_ci = mean.get('confidence_interval', {})
    median_val = median.get('point_estimate')
    std_val = std.get('point_estimate')
    lower = mean_ci.get('lower_bound')
    upper = mean_ci.get('upper_bound')
    report.append(f'| {name} | {mean_val:.3f} | {median_val:.3f} | {std_val:.3f} | {lower:.3f} | {upper:.3f} |')

out = '\n'.join(report)
print(out)
Path('target/criterion/bench_report.md').write_text(out)
print('\nWritten target/criterion/bench_report.md')
