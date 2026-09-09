(function () {
    var COOKIE = "csrf_token";
    var FIELD = "csrf";
    var HEADER = "X-CSRF-Token";
    var META = 'meta[name="csrf-token"]';

    function readToken() {
        var meta = document.querySelector(META);
        if (meta) {
            var fromMeta = (meta.getAttribute("content") || "").trim();
            if (fromMeta) return fromMeta;
        }
        var match = document.cookie.match(
            new RegExp("(?:^|; )" + COOKIE.replace(/([.*+?^${}()|[\]\\])/g, "\\$1") + "=([^;]*)")
        );
        return match ? decodeURIComponent(match[1]) : "";
    }

    function ensureFormToken(form) {
        if (!form || !form.tagName || form.tagName.toUpperCase() !== "FORM") return;
        var method = (form.getAttribute("method") || "get").toLowerCase();
        if (method !== "post" && method !== "put" && method !== "patch" && method !== "delete") {
            return;
        }
        var token = readToken();
        if (!token) return;
        var input = form.querySelector('input[name="' + FIELD + '"]');
        if (input) {
            input.value = token;
            return;
        }
        input = document.createElement("input");
        input.type = "hidden";
        input.name = FIELD;
        input.value = token;
        if (form.firstChild) {
            form.insertBefore(input, form.firstChild);
        } else {
            form.appendChild(input);
        }
    }

    function injectAllForms() {
        var forms = document.querySelectorAll("form");
        for (var i = 0; i < forms.length; i++) {
            ensureFormToken(forms[i]);
        }
    }

    function applyFetchHeaders(init, input) {
        init = init || {};
        var method = "GET";
        if (init.method) {
            method = String(init.method).toUpperCase();
        } else if (typeof Request !== "undefined" && input instanceof Request) {
            method = input.method.toUpperCase();
        }
        if (method === "GET" || method === "HEAD" || method === "OPTIONS") {
            return init;
        }
        var token = readToken();
        if (!token) return init;
        var headers = new Headers(
            init.headers || (input instanceof Request ? input.headers : undefined)
        );
        if (!headers.has(HEADER)) {
            headers.set(HEADER, token);
        }
        init.headers = headers;
        return init;
    }

    document.addEventListener(
        "submit",
        function (event) {
            ensureFormToken(event.target);
        },
        true
    );


    if (typeof HTMLFormElement !== "undefined") {
        var rawSubmit = HTMLFormElement.prototype.submit;
        HTMLFormElement.prototype.submit = function () {
            ensureFormToken(this);
            return rawSubmit.call(this);
        };
    }

    if (typeof window.fetch === "function") {
        var rawFetch = window.fetch;
        window.fetch = function (input, init) {
            return rawFetch.call(this, input, applyFetchHeaders(init, input));
        };
    }

    window.OakisCsrf = {
        token: readToken,
        ensureForm: ensureFormToken,
        headerName: HEADER,
        fieldName: FIELD,
    };

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", injectAllForms);
    } else {
        injectAllForms();
    }
})();
