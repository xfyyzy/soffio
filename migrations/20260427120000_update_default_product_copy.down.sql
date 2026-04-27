ALTER TABLE site_settings
    ALTER COLUMN footer_copy SET DEFAULT 'Stillness guides the wind; the wind reshapes stillness.',
    ALTER COLUMN meta_description SET DEFAULT 'Whispers on motion, balance, and form.',
    ALTER COLUMN og_description SET DEFAULT 'Traces of motion, balance, and form in continual drift.';

UPDATE site_settings
SET footer_copy = CASE
        WHEN footer_copy = 'Static output, focused admin workflows, and self-hosted control for technical writers.'
        THEN 'Stillness guides the wind; the wind reshapes stillness.'
        ELSE footer_copy
    END,
    meta_description = CASE
        WHEN meta_description = 'Soffio is a calm, self-hosted publishing system for technical writers who want static output, admin convenience, and operational control.'
        THEN 'Whispers on motion, balance, and form.'
        ELSE meta_description
    END,
    og_description = CASE
        WHEN og_description = 'Calm self-hosted publishing with static output, focused admin workflows, and explicit automation.'
        THEN 'Traces of motion, balance, and form in continual drift.'
        ELSE og_description
    END,
    updated_at = now()
WHERE id = 1
  AND (
      footer_copy = 'Static output, focused admin workflows, and self-hosted control for technical writers.'
      OR meta_description = 'Soffio is a calm, self-hosted publishing system for technical writers who want static output, admin convenience, and operational control.'
      OR og_description = 'Calm self-hosted publishing with static output, focused admin workflows, and explicit automation.'
  );
