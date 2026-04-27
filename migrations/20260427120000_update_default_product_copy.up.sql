ALTER TABLE site_settings
    ALTER COLUMN footer_copy SET DEFAULT 'Static output, focused admin workflows, and self-hosted control for technical writers.',
    ALTER COLUMN meta_description SET DEFAULT 'Soffio is a calm, self-hosted publishing system for technical writers who want static output, admin convenience, and operational control.',
    ALTER COLUMN og_description SET DEFAULT 'Calm self-hosted publishing with static output, focused admin workflows, and explicit automation.';

UPDATE site_settings
SET footer_copy = CASE
        WHEN footer_copy = 'Stillness guides the wind; the wind reshapes stillness.'
        THEN 'Static output, focused admin workflows, and self-hosted control for technical writers.'
        ELSE footer_copy
    END,
    meta_description = CASE
        WHEN meta_description = 'Whispers on motion, balance, and form.'
        THEN 'Soffio is a calm, self-hosted publishing system for technical writers who want static output, admin convenience, and operational control.'
        ELSE meta_description
    END,
    og_description = CASE
        WHEN og_description = 'Traces of motion, balance, and form in continual drift.'
        THEN 'Calm self-hosted publishing with static output, focused admin workflows, and explicit automation.'
        ELSE og_description
    END,
    updated_at = now()
WHERE id = 1
  AND (
      footer_copy = 'Stillness guides the wind; the wind reshapes stillness.'
      OR meta_description = 'Whispers on motion, balance, and form.'
      OR og_description = 'Traces of motion, balance, and form in continual drift.'
  );
