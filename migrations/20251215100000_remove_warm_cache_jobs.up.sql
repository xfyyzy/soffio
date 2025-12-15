-- Remove obsolete warm_cache jobs now that caching is disabled.
DELETE FROM apalis.jobs WHERE job_type = 'warm_cache';
