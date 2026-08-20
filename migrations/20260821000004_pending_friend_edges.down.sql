DELETE FROM realtime.realtime_friend_edges WHERE pending;
ALTER TABLE realtime.realtime_friend_edges DROP COLUMN pending;
