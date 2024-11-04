// Package time is a wrapper around the go standard time library.
package time

import (
	"time"
)

// Since returns the duration since t.
func Since(t time.Time) time.Duration {
	return Now().Sub(t)
}

// Until returns the duration until t.
func Until(t time.Time) time.Duration {
	return t.Sub(Now())
}

var currentTime int64

func SetCurrentTime(t int64) {
	currentTime = t
}

// Now returns the current local time.
func Now() time.Time {
	return time.Unix(currentTime, 0)
}
