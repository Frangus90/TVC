-- Add fallback ICS URL column and upgrade primary feeds to session-aware sources.
-- Several series in migration 012 pointed at TooMuchRacing Google Calendars, which
-- are race-only by design. This migration switches them to sportstimes/motorsportradar
-- feeds that include practice/qualifying timings, and corrects shared calendar IDs
-- that were misrouted (IMSA, NASCAR Xfinity/Truck, DTM, Supercars, Super GT, Extreme E).
ALTER TABLE racing_series ADD COLUMN fallback_ics_url TEXT;

-- F1: keep vidmar as primary; add motorsportcalendars as fallback
UPDATE racing_series SET
    ics_url = 'https://f1.vidmar.net/calendar.ics',
    fallback_ics_url = 'https://files-f1.motorsportcalendars.com/f1-calendar_p1_p2_p3_q_sprint_gp.ics'
WHERE slug = 'f1';

UPDATE racing_series SET
    ics_url = 'https://files-f2.motorsportcalendars.com/f2-calendar_q_sprint_feature.ics',
    fallback_ics_url = 'https://motorsportradar.com/cal/f2/2026.ics'
WHERE slug = 'f2';

UPDATE racing_series SET
    ics_url = 'https://files-f3.motorsportcalendars.com/f3-calendar_q_sprint_feature.ics',
    fallback_ics_url = 'https://motorsportradar.com/cal/f3/2026.ics'
WHERE slug = 'f3';

UPDATE racing_series SET
    ics_url = 'https://files-f1-academy.motorsportcalendars.com/f1-academy-calendar_fp1_fp2_qualifying1_qualifying2_race1_race2_race3.ics',
    fallback_ics_url = NULL
WHERE slug = 'f1-academy';

UPDATE racing_series SET
    ics_url = 'https://files-fe.motorsportcalendars.com/fe-calendar_p1_p2_p3_qualifying_race.ics',
    fallback_ics_url = 'https://motorsportradar.com/cal/fe/2026.ics'
WHERE slug = 'formula-e';

UPDATE racing_series SET
    ics_url = 'https://files-indycar.motorsportcalendars.com/indycar-calendar_p1_p2_p3_qualifying_warmup_race.ics',
    fallback_ics_url = 'https://motorsportradar.com/cal/indycar/2026.ics'
WHERE slug = 'indycar';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/super-formula/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/n00q1ce78ahlgmjn9qvs8nc6ko%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'super-formula';

-- MotoGP stack: nixxo already has sessions; add motorsportradar fallback
UPDATE racing_series SET
    fallback_ics_url = 'https://motorsportradar.com/cal/motogp/2026.ics'
WHERE slug = 'motogp';

UPDATE racing_series SET
    fallback_ics_url = 'https://motorsportradar.com/cal/moto-2/2026.ics'
WHERE slug = 'moto2';

UPDATE racing_series SET
    fallback_ics_url = 'https://motorsportradar.com/cal/moto-3/2026.ics'
WHERE slug = 'moto3';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/worldsbk/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/0rts2iu5gd88eis52c084ltlhc%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'wsbk';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/wec/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/61jccgg4rshh1temqk0dj4lens%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'wec';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/imsa/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/njulhksvo83qeoruc3nhend9js%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'imsa';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/nascar-cup/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'nascar-cup';

UPDATE racing_series SET
    ics_url = 'https://calendar.google.com/calendar/ical/po54lfbfrvlkrrhirlame40a6c%40group.calendar.google.com/public/basic.ics',
    fallback_ics_url = NULL
WHERE slug = 'nascar-xs';

UPDATE racing_series SET
    ics_url = 'https://calendar.google.com/calendar/ical/lnpvdfud8lhom7opdsnnbtu268%40group.calendar.google.com/public/basic.ics',
    fallback_ics_url = NULL
WHERE slug = 'nascar-truck';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/wrc/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/fei68gpe16c85ed3jjdtvrn8ns%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'wrc';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/dtm/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/0urnjij5qqj3ijoht52fdsqk18%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'dtm';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/supercars/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/sd36enn936vc7dv35ucgulbjng%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'v8supercars';

UPDATE racing_series SET
    ics_url = 'https://motorsportradar.com/cal/super-gt/2026.ics',
    fallback_ics_url = 'https://calendar.google.com/calendar/ical/5ni9rjbofnkfvmpidmjpep9ek0%40group.calendar.google.com/public/basic.ics'
WHERE slug = 'supergt';

UPDATE racing_series SET
    ics_url = 'https://calendar.google.com/calendar/ical/4th6rmpe52qpjvmfuq4p41vgvo%40group.calendar.google.com/public/basic.ics',
    fallback_ics_url = NULL
WHERE slug = 'extreme-e';
