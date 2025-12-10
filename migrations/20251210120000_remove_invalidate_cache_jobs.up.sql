-- Remove obsolete cache invalidation jobs now handled synchronously.
DELETE FROM apalis.jobs WHERE job_type = 'invalidate_cache';
