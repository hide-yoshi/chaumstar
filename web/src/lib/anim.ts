/** Tiny delay helper used to pace terminal output animations. */
export function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Default cadence for sequential terminal lines (milliseconds). */
export const LINE_DELAY = 90;
