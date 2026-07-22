// Dedicated Batch 5 entry point. The accepted product scenario owns the real UI
// workflow; SUMI_BATCH5_DURATION_SECONDS turns on its sustained hardening phase.
if (!process.env.SUMI_BATCH5_DURATION_SECONDS) process.env.SUMI_BATCH5_DURATION_SECONDS = '1800';
await import('./product-uat.mjs');
