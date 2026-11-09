# Security policy — pre-implementation

There is no released Flow security engine. Treat all guarantees as proposed until tested. Do not run malicious hooks or tests with real secrets or on the live Ravum workspace. Use only artificial canaries and disposable fixtures **after** a scoped `GO_R0_FIXTURES`.

Do not disclose credentials or private paths in public issues; coordinate private reports with repo owner through a secure channel / GitHub private vulnerability reporting if enabled. Historic Ravum P0-TREE/P0-HOOK-ORDER/P0-ENV are code-reading findings, not dynamically reproduced exploits.
