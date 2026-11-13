-- Dashboard layout v2: a section can take the full width, half or a third of the board.
ALTER TABLE dashboard_sections
    ADD COLUMN width TEXT NOT NULL DEFAULT 'full'
        CHECK (width IN ('full', 'half', 'third'));
