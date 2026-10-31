-- Milestone 2: personal dashboard (sectioned bookmark grid), ready for M3 sharing.

CREATE TABLE dashboards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT 'Home',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX dashboards_one_default_per_user
    ON dashboards (owner_user_id)
    WHERE is_default;

CREATE INDEX dashboards_owner_idx ON dashboards (owner_user_id);

CREATE TABLE dashboard_sections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dashboard_id UUID NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX dashboard_sections_dashboard_idx ON dashboard_sections (dashboard_id);

CREATE TABLE dashboard_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_id UUID NOT NULL REFERENCES dashboard_sections(id) ON DELETE CASCADE,
    bookmark_id UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    position INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (section_id, bookmark_id)
);

CREATE INDEX dashboard_items_section_idx ON dashboard_items (section_id);
CREATE INDEX dashboard_items_bookmark_idx ON dashboard_items (bookmark_id);
