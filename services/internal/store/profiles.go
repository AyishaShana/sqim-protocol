package store

import (
	"context"
	"time"

	"github.com/jackc/pgx/v5"
)

type UserProfile struct {
	Address               string    `json:"address"`
	DisplayName           string    `json:"display_name"`
	Bio                   string    `json:"bio"`
	AvatarURL             string    `json:"avatar_url"`
	NotificationFrequency string    `json:"notification_frequency"`
	DriftThresholdBPS     int       `json:"drift_threshold_bps"`
	NotificationEmail     string    `json:"notification_email"`
	UpdatedAt             time.Time `json:"updated_at"`
}

type ProfileChallenge struct {
	Address   string
	Nonce     string
	Message   string
	ExpiresAt time.Time
}

func (s *Store) Profile(ctx context.Context, address string) (UserProfile, error) {
	var profile UserProfile
	err := s.pool.QueryRow(ctx, `
		select address, display_name, bio, avatar_url, notification_frequency,
		       drift_threshold_bps, notification_email, updated_at
		from user_profiles
		where address = $1
	`, address).Scan(&profile.Address, &profile.DisplayName, &profile.Bio, &profile.AvatarURL,
		&profile.NotificationFrequency, &profile.DriftThresholdBPS, &profile.NotificationEmail,
		&profile.UpdatedAt)
	return profile, err
}

func (s *Store) SaveProfile(ctx context.Context, profile UserProfile) (UserProfile, error) {
	_, err := s.pool.Exec(ctx, `
		insert into user_profiles
			(address, display_name, bio, avatar_url, notification_frequency,
			 drift_threshold_bps, notification_email, email, updated_at)
		values ($1, $2, $3, $4, $5, $6, $7, $7, now())
		on conflict (address) do update
		set display_name = excluded.display_name,
		    bio = excluded.bio,
		    avatar_url = excluded.avatar_url,
		    notification_frequency = excluded.notification_frequency,
		    drift_threshold_bps = excluded.drift_threshold_bps,
		    notification_email = excluded.notification_email,
		    email = excluded.email,
		    updated_at = now()
	`, profile.Address, profile.DisplayName, profile.Bio, profile.AvatarURL,
		profile.NotificationFrequency, profile.DriftThresholdBPS, profile.NotificationEmail)
	if err != nil {
		return UserProfile{}, err
	}
	return s.Profile(ctx, profile.Address)
}

func (s *Store) CreateProfileChallenge(ctx context.Context, challenge ProfileChallenge) error {
	_, err := s.pool.Exec(ctx, `
		delete from profile_auth_challenges
		where expires_at < now() or used_at is not null
	`)
	if err != nil {
		return err
	}
	_, err = s.pool.Exec(ctx, `
		insert into profile_auth_challenges (nonce, address, message, expires_at)
		values ($1, $2, $3, $4)
	`, challenge.Nonce, challenge.Address, challenge.Message, challenge.ExpiresAt)
	return err
}

func (s *Store) ProfileChallenge(ctx context.Context, address, nonce string) (ProfileChallenge, error) {
	var challenge ProfileChallenge
	err := s.pool.QueryRow(ctx, `
		select address, nonce, message, expires_at
		from profile_auth_challenges
		where address = $1 and nonce = $2 and used_at is null and expires_at > now()
	`, address, nonce).Scan(&challenge.Address, &challenge.Nonce, &challenge.Message, &challenge.ExpiresAt)
	return challenge, err
}

func (s *Store) ConsumeProfileChallenge(ctx context.Context, address, nonce string) error {
	command, err := s.pool.Exec(ctx, `
		update profile_auth_challenges
		set used_at = now()
		where address = $1 and nonce = $2 and used_at is null and expires_at > now()
	`, address, nonce)
	if err != nil {
		return err
	}
	if command.RowsAffected() != 1 {
		return pgx.ErrNoRows
	}
	return nil
}
