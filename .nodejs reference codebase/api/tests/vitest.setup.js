/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { mkdirSync } from 'fs';
import { resolve } from 'path';

mkdirSync(resolve(process.cwd(), 'coverage', '.tmp'), { recursive: true });
