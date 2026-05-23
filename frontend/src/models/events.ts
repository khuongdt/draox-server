import { useState, useCallback } from 'react';

/** Maximum number of events retained in the FIFO buffer. */
const MAX_EVENTS = 500;

/**
 * FIFO event buffer for the live server event stream.
 * New events are prepended; the buffer is capped at MAX_EVENTS.
 * Supports pause/resume to freeze the display without losing events.
 */
export default function useEventsModel() {
  const [events, setEvents] = useState<API.ServerEvent[]>([]);
  const [paused, setPaused] = useState(false);

  /** Prepend an event to the buffer (ignored when paused). */
  const addEvent = useCallback(
    (event: API.ServerEvent) => {
      if (paused) return;
      setEvents((prev) => {
        const next = [event, ...prev];
        return next.length > MAX_EVENTS ? next.slice(0, MAX_EVENTS) : next;
      });
    },
    [paused],
  );

  /** Remove all events from the buffer. */
  const clear = useCallback(() => setEvents([]), []);

  /** Toggle between paused and live states. */
  const togglePause = useCallback(() => setPaused((p) => !p), []);

  return { events, paused, addEvent, clear, togglePause };
}
