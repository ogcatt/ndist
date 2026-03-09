// dev-hotreload-fix.js — only injected in `dx serve` via [web.resource.dev]
//
// Root cause: when a fullstack rebuild finishes, dioxus-web calls
// `window.location.reload()` immediately. The dx proxy then forwards the
// request to the new backend process, which hasn't finished binding its port
// yet, so dx returns a raw "Backend connection failed" HTTP 500 page.
// Because that page has no scripts, nothing auto-retries.
//
// Fix: replace Location.prototype.reload with a version that first confirms
// the backend is up (i.e. GET / returns something other than the dx 500 error
// page) before performing the real reload.

(function () {
    let reloadPending = false;

    let _origReload;
    try {
        _origReload = Location.prototype.reload;
        Location.prototype.reload = function () {
            if (reloadPending) return;
            reloadPending = true;
            const loc = this;
            waitForBackend(30, 600)
                .then(function () { _origReload.call(loc); })
                .catch(function () { _origReload.call(loc); });
            // Return undefined (void) — satisfies web-sys binding expectation.
        };
    } catch (e) {
        // Browser didn't allow override — silent no-op, dev experience unchanged.
        console.warn('[dev-hotreload-fix] Could not patch Location.prototype.reload:', e);
    }

    // Resolves once GET / returns a status that isn't the dx "backend down" 500.
    async function waitForBackend(retries, delayMs) {
        for (let i = 0; i < retries; i++) {
            try {
                const r = await fetch('/', { cache: 'no-store' });
                if (r.status !== 500) return;
                const text = await r.text();
                if (!text.includes('Backend connection failed')) return;
            } catch (_) {
                // Network-level failure — backend still starting.
            }
            await sleep(delayMs);
        }
        // Exhausted retries — let the reload happen anyway.
    }

    function sleep(ms) {
        return new Promise(function (resolve) { setTimeout(resolve, ms); });
    }
})();
