/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Startup banner utility for visual flair.
 * @module utils/banner
 */

const BANNER = `
\x1b[36m   ______            __                  \x1b[0m
\x1b[36m  / ____/___ _____  / /____  ____  ____ _\x1b[0m
\x1b[36m / / __/ __ \`/ __ \\/ __/ _ \\/ __ \\/ __ \`/\x1b[0m
\x1b[36m/ /_/ / /_/ / / / / /_/  __/ / / / /_/ / \x1b[0m
\x1b[36m\\____/\\__,_/_/ /_/\\__/\\___/_/ /_/\\__, /  \x1b[0m
\x1b[36m                                /____/   \x1b[0m
\x1b[35m    __  ___      __        _                 __\x1b[0m
\x1b[35m   /  |/  /___ _/ /_______(_)___ ___  ____ _/ /\x1b[0m
\x1b[35m  / /|_/ / __ \`/ //_/ ___/ / __ \`__ \\/ __ \`/ / \x1b[0m
\x1b[35m / /  / / /_/ / ,< (__  ) / / / / / / /_/ / /  \x1b[0m
\x1b[35m/_/  /_/\\__,_/_/|_/____/_/_/ /_/ /_/\\__,_/_/   \x1b[0m

\x1b[90m      >> Smart Browser Automation <<      \x1b[0m
`;

/**
 * Prints the startup banner
 */
export function showBanner() {
  console.log(BANNER);
}
