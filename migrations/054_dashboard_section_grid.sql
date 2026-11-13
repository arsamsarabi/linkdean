-- Bento dashboard: each section has a place and a size on a 12-column grid, in grid units,
-- replacing the full / half / third width from 053. Rows are a fixed height; the web app
-- packs sections upward, so y only needs to keep their order.

ALTER TABLE dashboard_sections
    ADD COLUMN grid_x INT NOT NULL DEFAULT 0 CHECK (grid_x BETWEEN 0 AND 11),
    ADD COLUMN grid_y INT NOT NULL DEFAULT 0 CHECK (grid_y BETWEEN 0 AND 10000),
    ADD COLUMN grid_w INT NOT NULL DEFAULT 12 CHECK (grid_w BETWEEN 3 AND 12),
    ADD COLUMN grid_h INT NOT NULL DEFAULT 8 CHECK (grid_h BETWEEN 3 AND 30),
    ADD CONSTRAINT dashboard_sections_grid_fits CHECK (grid_x + grid_w <= 12);

-- Lay existing sections out left to right in their order, wrapping rows, at their old width
-- and a height that suits their display.
DO $$
DECLARE
    r RECORD;
    cur UUID;
    cx INT;
    cy INT;
    w INT;
    h INT;
BEGIN
    FOR r IN SELECT id, dashboard_id, width, display FROM dashboard_sections ORDER BY dashboard_id, position LOOP
        IF cur IS DISTINCT FROM r.dashboard_id THEN
            cur := r.dashboard_id;
            cx := 0;
            cy := 0;
        END IF;
        w := CASE r.width WHEN 'half' THEN 6 WHEN 'third' THEN 4 ELSE 12 END;
        h := CASE r.display WHEN 'icons' THEN 5 WHEN 'small' THEN 8 ELSE 11 END;
        IF cx + w > 12 THEN
            cx := 0;
            cy := cy + 11;
        END IF;
        UPDATE dashboard_sections SET grid_x = cx, grid_y = cy, grid_w = w, grid_h = h WHERE id = r.id;
        cx := cx + w;
    END LOOP;
END $$;

ALTER TABLE dashboard_sections DROP COLUMN width;
