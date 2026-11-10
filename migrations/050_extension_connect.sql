-- Browser extension sign-in: a signed-in person approves the extension on the web app, which
-- hands the extension a single-use code; the extension trades it, with its PKCE verifier, for
-- an API key. Only the code's SHA-256 hash is stored, and codes live for two minutes.
-- extension_origin is the extension's own base address (chrome-extension://<id>/ or
-- moz-extension://<uuid>/); the trade must name the same one.
CREATE TABLE extension_connect_codes (
    code_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- base64url(SHA-256(code_verifier)), without padding (PKCE S256).
    code_challenge TEXT NOT NULL,
    extension_origin TEXT NOT NULL,
    -- Name of the API key the code becomes, such as "Browser extension · Chrome".
    key_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX extension_connect_codes_expires_idx ON extension_connect_codes (expires_at);
