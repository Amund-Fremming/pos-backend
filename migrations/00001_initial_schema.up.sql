CREATE TABLE IF NOT EXISTS "user_data" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    home_time TIME NOT NULL,
    home_lat DOUBLE PRECISION NOT NULL,
    home_lon DOUBLE PRECISION NOT NULL,
    home_display VARCHAR (150),
    work_time TIME NOT NULL,
    work_lat DOUBLE PRECISION NOT NULL,
    work_lon DOUBLE PRECISION NOT NULL,
    work_display VARCHAR (150),
    commute_minutes INTEGER NOT NULL,
    alert_days BIT(7) NOT NULL DEFAULT B'0000000',
    push_token VARCHAR(200) UNIQUE,
    last_alerted_date DATE
);
