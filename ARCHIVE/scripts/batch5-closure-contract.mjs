const date = value => value ? String(value).slice(0, 10) : null;

export function validateFutureBoundary(sample) {
  const failures = [];
  const expectedCount = sample.authoritativeIndex + 1;
  const boundary = date(sample.authoritativeDate);
  const equal = (surface, actual, expected) => { if (actual !== expected) failures.push({ surface, actual, expected }); };
  const atOrBefore = (surface, actual) => { if (actual && actual > boundary) failures.push({ surface, actual, expected: `<=${boundary}` }); };
  equal('practice.index', sample.practiceIndex, sample.authoritativeIndex);
  equal('api.count', sample.apiCandleCount, expectedCount);
  equal('api.maxDate', date(sample.apiMaxDate), boundary);
  equal('chart.count', sample.chartCandleCount, expectedCount);
  equal('chart.maxDate', date(sample.chartMaxDate), boundary);
  for (const indicator of sample.indicators ?? []) {
    equal(`indicator.${indicator.id}.inputMaxDate`, date(indicator.inputMaxDate), boundary);
    equal(`indicator.${indicator.id}.responseMaxDate`, date(indicator.responseMaxDate), boundary);
    for (const [series, maxDate] of Object.entries(indicator.seriesMaxDates ?? {})) {
      equal(`indicator.${indicator.id}.chart.${series}`, date(maxDate), boundary);
    }
  }
  atOrBefore('markers.maxDate', date(sample.markerMaxDate));
  for (const drawing of sample.visibleProviderDrawings ?? []) {
    for (const anchorDate of drawing.anchorDates ?? []) atOrBefore(`drawing.${drawing.id}.anchor`, date(anchorDate));
  }
  for (const drawing of sample.retainedFutureDrawings ?? []) {
    if (drawing.providerVisible) failures.push({ surface: `drawing.${drawing.id}.retained-future-visible`, actual: true, expected: false });
  }
  return { pass: failures.length === 0, failures };
}
