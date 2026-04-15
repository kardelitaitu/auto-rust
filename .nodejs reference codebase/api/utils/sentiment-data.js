/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Comprehensive Sentiment Analysis Data & Lexicons
 * Extensive word lists, patterns, emotions, and contextual markers
 * @module utils/sentiment-data
 */

// ============================================================================
// POSITIVE LEXICON (Valence - Comprehensive)
// ============================================================================
export const POSITIVE_LEXICON = {
    // Strong positive
    strong: [
        'love',
        'adore',
        'amazing',
        'awesome',
        'wonderful',
        'fantastic',
        'incredible',
        'excellent',
        'brilliant',
        'perfect',
        'genius',
        'masterpiece',
        'stunning',
        'gorgeous',
        'beautiful',
        'breathtaking',
        'magnificent',
        'extraordinary',
        'outstanding',
        'phenomenal',
        'superb',
        'outstanding',
        'remarkable',
        'exceptional',
        'splendid',
        'marvelous',
        'tremendous',
        'fabulous',
        'divine',
        'sublime',
        'glorious',
    ],

    // Moderate positive
    moderate: [
        'good',
        'great',
        'nice',
        'fun',
        'happy',
        'excited',
        'cool',
        'awesome',
        'sweet',
        'lovely',
        'delightful',
        'charming',
        'pleasant',
        'enjoyable',
        'satisfying',
        'rewarding',
        'interesting',
        'engaging',
        'wonderful',
        'positive',
        'uplifting',
        'inspiring',
        'motivating',
        'encouraging',
        'helpful',
        'useful',
        'beneficial',
        'worthwhile',
    ],

    // Weak/mild positive
    weak: [
        'ok',
        'fine',
        'decent',
        'alright',
        'acceptable',
        'pleasant',
        'neat',
        'tidy',
        'reasonable',
        'satisfactory',
        'adequate',
        'competent',
        'capable',
        'workable',
        'viable',
        'feasible',
    ],

    // Achievement & success words
    achievement: [
        'accomplished',
        'achieved',
        'succeeded',
        'won',
        'triumph',
        'victory',
        'breakthrough',
        'milestone',
        'record',
        'goal',
        'accomplished',
        'earned',
        'attained',
        'reached',
        'completed',
        'finished',
        'delivered',
        'succeeded',
        'prospered',
        'thrived',
        'flourished',
        'excelled',
        'outperformed',
        'surpassed',
    ],

    // Love & affection
    love: [
        'love',
        'adore',
        'cherish',
        'treasure',
        'devoted',
        'affectionate',
        'caring',
        'tender',
        'passionate',
        'romantic',
        'enamored',
        'fond',
        'smitten',
        'head over heels',
        'besotted',
        'infatuated',
    ],

    // Appreciation & gratitude
    grateful: [
        'thank',
        'thanks',
        'appreciate',
        'grateful',
        'blessed',
        'thankful',
        'gratitude',
        'recognition',
        'acknowledgment',
        'honor',
        'privilege',
        'indebted',
        'beholden',
        'appreciative',
        'obliged',
    ],

    // Trust & reliability
    trust: [
        'trust',
        'reliable',
        'dependable',
        'faithful',
        'loyal',
        'honest',
        'trustworthy',
        'genuine',
        'authentic',
        'sincere',
        'true',
        'real',
        'solid',
        'steadfast',
        'committed',
        'dedicated',
    ],

    // Beauty & aesthetics
    beauty: [
        'beautiful',
        'gorgeous',
        'stunning',
        'elegant',
        'graceful',
        'aesthetic',
        'artistic',
        'refined',
        'sophisticated',
        'classy',
        'stylish',
        'fashionable',
        'chic',
        'sleek',
        'striking',
    ],

    // Energy & vitality
    energy: [
        'energetic',
        'vibrant',
        'lively',
        'spirited',
        'dynamic',
        'active',
        'vigorous',
        'animated',
        'enthusiastic',
        'exuberant',
        'buoyant',
        'invigorating',
        'stimulating',
        'thrilling',
        'exciting',
    ],

    // Wisdom & intelligence
    wise: [
        'wise',
        'intelligent',
        'brilliant',
        'clever',
        'smart',
        'insightful',
        'thoughtful',
        'philosophical',
        'intellectual',
        'knowledgeable',
        'enlightened',
        'sage',
        'astute',
        'discerning',
        'perceptive',
    ],
};

// ============================================================================
// NEGATIVE LEXICON (Valence - Comprehensive)
// ============================================================================
export const NEGATIVE_LEXICON = {
    // Strong negative
    strong: [
        'hate',
        'despise',
        'abhor',
        'detest',
        'horrible',
        'terrible',
        'catastrophic',
        'disastrous',
        'atrocious',
        'appalling',
        'vile',
        'revolting',
        'repulsive',
        'disgusting',
        'abominable',
        'loathsome',
        'ghastly',
        'execrable',
        'detestable',
        'odious',
        'heinous',
    ],

    // Moderate negative
    moderate: [
        'bad',
        'sad',
        'angry',
        'frustrated',
        'annoyed',
        'upset',
        'disappointed',
        'distressed',
        'worried',
        'anxious',
        'concerned',
        'unhappy',
        'miserable',
        'depressed',
        'disheartened',
        'discouraged',
        'dismayed',
        'discontented',
        'dissatisfied',
        'displeased',
    ],

    // Mild/weak negative
    weak: [
        'meh',
        'blah',
        'dull',
        'boring',
        'mediocre',
        'plain',
        'bland',
        'mundane',
        'uninteresting',
        'tiresome',
        'tedious',
        'drab',
        'humdrum',
        'tedious',
        'wearisome',
        'monotonous',
    ],

    // Tragedy & death
    tragedy: [
        'died',
        'death',
        'rip',
        'passed away',
        'gone',
        'lost',
        'fatal',
        'fatality',
        'mortality',
        'deceased',
        'perished',
        'expired',
        'succumbed',
        'casualties',
        'deaths',
        'killed',
        'slain',
        'murdered',
        'homicide',
        'manslaughter',
        'infanticide',
        'euthanasia',
    ],

    // Grief & sorrow
    grief: [
        'mourning',
        'grieving',
        'heartbroken',
        'devastated',
        'anguish',
        'sorrow',
        'despair',
        'misery',
        'suffering',
        'agony',
        'torment',
        'distress',
        'anguished',
        'bereaved',
        'inconsolable',
        'despondent',
        'forlorn',
        'dejected',
        'melancholy',
        'woeful',
        'sorrowful',
    ],

    // Violence & harm
    violence: [
        'hate',
        'war',
        'fight',
        'attack',
        'battle',
        'violence',
        'assault',
        'threat',
        'danger',
        'crisis',
        'emergency',
        'peril',
        'hazard',
        'jeopardy',
        'risk',
        'wounded',
        'injured',
        'maimed',
        'brutality',
        'cruelty',
        'torture',
        'abuse',
        'rape',
        'sexual assault',
    ],

    // Scam & fraud
    scam: [
        'scam',
        'hacked',
        'stolen',
        'fraud',
        'fake',
        'phishing',
        'malware',
        'virus',
        'security breach',
        'compromised',
        'breach',
        'exposed',
        'leaked',
        'pirated',
        'counterfeit',
        'forged',
        'con',
        'swindle',
        'embezzlement',
        'theft',
        'robbery',
        'burglary',
    ],

    // Controversy & scandal
    controversy: [
        'scandal',
        'controversy',
        'accused',
        'allegations',
        'lawsuit',
        'sued',
        'investigation',
        'subpoena',
        'raid',
        'charged',
        'indicted',
        'guilty',
        'convicted',
        'sentenced',
        'crime',
        'criminal',
        'felon',
        'conviction',
        'offense',
        'violation',
        'breach',
        'infraction',
        'misconduct',
    ],

    // Failure & loss
    failure: [
        'failed',
        'failure',
        'collapse',
        'bankruptcy',
        'bankrupt',
        'ruin',
        'ruined',
        'destroyed',
        'devastated',
        'demolished',
        'wrecked',
        'shattered',
        'broken',
        'broken down',
        'lost',
        'defeat',
        'defeated',
        'loss',
        'losing',
        'quit',
        'gave up',
        'surrender',
    ],

    // Pain & injury
    pain: [
        'pain',
        'hurt',
        'injury',
        'injured',
        'wound',
        'wounded',
        'ill',
        'illness',
        'sick',
        'disease',
        'cancer',
        'terminal',
        'disabled',
        'disability',
        'impaired',
        'suffering',
        'ache',
        'agony',
        'torment',
        'excruciating',
        'unbearable',
        'intense',
        'severe',
        'critical',
    ],

    // Betrayal & trust issues
    betrayal: [
        'betrayal',
        'betrayed',
        'backstab',
        'backstabbed',
        'traitor',
        'treachery',
        'unfaithful',
        'cheated',
        'cheater',
        'liar',
        'lie',
        'deception',
        'deceived',
        'manipulated',
        'used',
        'exploited',
        'abandoned',
    ],
};

// ============================================================================
// AROUSAL MARKERS (Excitement/Energy Level - Expanded)
// ============================================================================
export const AROUSAL_MARKERS = {
    high: {
        markers: [
            '!!!',
            'omg',
            'omgggg',
            'ahhhhh',
            'yesssss',
            'sooo',
            'literally',
            "can't even",
            'dying',
            'screaming',
            'crying',
            'wtf',
            'no way',
            'shut up',
            'you serious',
            'for real',
            'no joke',
            "i'm dead",
            "i can't",
            'unbelievable',
            'insane',
            'crazy',
            'wild',
            'outrageous',
        ],
        allCapsWeight: 0.2,
        exclamationWeight: 0.15,
        emojis: [
            '😍',
            '🤩',
            '😱',
            '🔥',
            '⚡',
            '💥',
            '🎉',
            '🙌',
            '😭',
            '😂',
            '🤣',
            '💀',
            '🤡',
            '🎭',
            '😆',
            '😄',
        ],
    },

    moderate: {
        markers: [
            'excited',
            'interesting',
            'cool',
            'nice',
            'good',
            'awesome',
            'fun',
            'entertaining',
            'engaging',
            'compelling',
            'captivating',
            'fascinating',
            'intriguing',
            'impressive',
            'notable',
        ],
        emojis: ['😊', '👍', '😌', '💭', '👀', '🧐', '😐'],
    },

    low: {
        markers: [
            'calmly',
            'gently',
            'quietly',
            'softly',
            'peaceful',
            'tranquil',
            'serene',
            'meditative',
            'reflective',
            'contemplative',
            'thoughtful',
            'relaxed',
            'composed',
            'placid',
            'still',
            'subdued',
            'hushed',
            'muffled',
            'whispered',
            'solemn',
            'grave',
        ],
        emojis: ['😌', '🤐', '🧘', '☕', '🌙', '😴', '🤫'],
    },
};

// ============================================================================
// DOMINANCE MARKERS (Assertiveness - Expanded)
// ============================================================================
export const DOMINANCE_MARKERS = {
    assertive: {
        words: [
            'must',
            'should',
            'need',
            'have to',
            'everyone knows',
            'obviously',
            'clearly',
            'definitely',
            'absolutely',
            'no question',
            'fact is',
            'believe me',
            'trust me',
            'i promise',
            'guaranteed',
            "won't",
            "can't",
            'demand',
            'require',
            'insist',
            'command',
            'order',
            'will',
            'shall',
            'must',
            'dare',
            'challenge',
        ],
        patterns: [
            /^[A-Z].*[.!]$/,
            /\b(always|never|all|every|none|no one)\b/i,
            /calling (out|them|it|bs)/i,
            /wake up/i,
            /listen up/i,
            /pay attention/i,
            /needs to change/i,
            /gotta/i,
        ],
    },

    submissive: {
        words: [
            'maybe',
            'perhaps',
            'possibly',
            'might',
            'could',
            'probably',
            'sorry',
            'apologize',
            'excuse me',
            'humble',
            'i think',
            'in my opinion',
            'just my two cents',
            'idk',
            'not sure',
            'i believe',
            'seems like',
            'appears to',
            'i guess',
            'sort of',
            'kind of',
            'somewhat',
            'fairly',
            'rather',
            'quite',
        ],
        patterns: [
            /\b(i think|i believe|in my opinion|just imo|imo|imho)\b/i,
            /\?$/,
            /\b(if|when|unless|provided)\b/i,
            /\b(might|could|may|possibly|perhaps)\b/i,
            /tentatively/i,
            /hesitantly/i,
        ],
    },

    neutral: {
        words: [
            'is',
            'appears',
            'seems',
            'suggests',
            'indicates',
            'means',
            'represents',
            'shows',
            'demonstrates',
            'reveals',
            'portrays',
            'depicts',
            'illustrates',
            'exemplifies',
            'exhibits',
        ],
    },
};

// ============================================================================
// SARCASM & IRONY MARKERS (Expanded)
// ============================================================================
export const SARCASM_MARKERS = {
    explicit: {
        markers: [
            'sure',
            'yeah right',
            'oh great',
            'wonderful',
            'fantastic',
            'because that worked well',
            'groundbreaking',
            'revolutionary',
            "well that's just",
            'thanks for that',
            'oh perfect',
            'brilliant idea',
            'genius move',
            'real smart',
            'wow smart',
            'well done',
            'good one',
            "that'll help",
            'really helpful',
            'super useful',
            'just what i needed',
            'oh wonderful',
            'how delightful',
            'truly inspiring',
            'what an honor',
        ],
        emojis: ['🙄', '😒', '🤐', '🤡', '🎭', '👏', '💅', '🤦'],
    },

    contradiction: {
        patterns: [
            /amazing.*terrible/i,
            /love.*hate/i,
            /best.*worst/i,
            /great.*sucks/i,
            /awesome.*awful/i,
            /beautiful.*ugly/i,
            /perfect.*broken/i,
            /intelligent.*dumb/i,
            /clean.*dirty/i,
            /safe.*dangerous/i,
        ],
    },

    context_inversion: {
        examples: [
            { pattern: /✨ tragedy ✨/i, confidence: 0.9 },
            { pattern: /goals/i, context: 'negative_sentiment', confidence: 0.7 },
            { pattern: /living my best life/i, context: 'sarcastic_situation', confidence: 0.6 },
            { pattern: /can't wait/i, context: 'clearly reluctant', confidence: 0.7 },
            { pattern: /so excited/i, context: 'obviously not', confidence: 0.6 },
        ],
    },

    dry_humor: {
        markers: [
            'apparently',
            'supposedly',
            'allegedly',
            'allegedly so',
            'yep',
            'sure thing',
            'absolutely',
            'by all means',
            'do please',
            'by my calculations',
            'clearly',
            'obviously',
            'naturally',
        ],
    },
};

// ============================================================================
// URGENCY MARKERS (Time-Sensitivity - Expanded)
// ============================================================================
export const URGENCY_MARKERS = {
    urgent: [
        'breaking',
        'urgent',
        'asap',
        'now',
        'immediately',
        'emergency',
        'crisis',
        'alert',
        'quick',
        'hurry',
        'deadline',
        'limited',
        'restricted',
        'only today',
        'ending soon',
        'last chance',
        'final call',
        'act now',
        "don't wait",
        'go now',
        'rush',
        'immediate action',
        'time sensitive',
        'urgent matter',
        'pressing',
        'critical',
    ],

    timeSensitive: [
        'election',
        'deadline',
        'launch',
        'release',
        'live',
        'happening',
        'starting',
        'begins',
        'coming soon',
        'countdown',
        'exclusive',
        'premiere',
        'debut',
        'premiere',
        'opening',
        'closing',
        'expires',
        'valid until',
        'while supplies last',
        'stock limited',
    ],

    scheduled: [
        'today',
        'tomorrow',
        'tonight',
        'this week',
        'next week',
        'this month',
        'tomorrow at',
        'this friday',
        'coming up',
        'upcoming',
        'scheduled',
        'planned',
        'set for',
        'in',
        'hours from now',
        'minutes from now',
    ],

    relaxed: [
        'soon',
        'eventually',
        'whenever',
        'someday',
        'later',
        'later this year',
        'no rush',
        'take your time',
        'whenever you want',
        'at your own pace',
        'no pressure',
        'no worries',
        'leisurely',
        'at your leisure',
    ],
};

// ============================================================================
// TOXICITY MARKERS (Aggression/Hostility - Expanded)
// ============================================================================
export const TOXICITY_MARKERS = {
    slurs_insults: [
        'stupid',
        'idiot',
        'dumb',
        'moron',
        'imbecile',
        'retard',
        'loser',
        'pathetic',
        'worthless',
        'useless',
        'trash',
        'garbage',
        'scum',
        'asshole',
        'bastard',
        'bitch',
        'amateur',
        'incompetent',
        'inept',
        'hopeless',
        'clueless',
    ],

    hostility: [
        'hate',
        'kill',
        'murder',
        'die',
        'suffer',
        'deserve it',
        'blocked',
        'reported',
        'canceled',
        'needs to die',
        'kill yourself',
        'kys',
        'rope',
        'jump',
        'end it',
        'take yourself out',
        "shouldn't exist",
        "doesn't deserve to live",
    ],

    personalAttacks: [
        /you.*stupid/i,
        /you.*hate/i,
        /your mother/i,
        /you deserve/i,
        /fuck you/i,
        /go to hell/i,
        /piece of shit/i,
        /go die/i,
        /kill yourself/i,
        /you suck/i,
        /you're an idiot/i,
        /what an asshole/i,
    ],

    aggression: {
        markers: [
            'destroy',
            'attack',
            'assault',
            'violence',
            'rage',
            'fury',
            'massacre',
            'slaughter',
            'genocide',
            'barbaric',
            'savage',
            'brutal',
            'vicious',
            'ferocious',
            'relentless',
            'merciless',
            'bloodthirsty',
            'violent',
            'aggressive',
            'hostile',
            'antagonistic',
        ],
        intensity: 0.9,
    },

    dehumanization: [
        'not human',
        'subhuman',
        'animal',
        'beast',
        'creature',
        'vermin',
        'pest',
        'plague',
        'disease',
        'infection',
        'cancer',
        'tumor',
        'parasite',
        'infestation',
    ],
};

// ============================================================================
// EMOJI SENTIMENT DICTIONARY (Comprehensive)
// ============================================================================
export const EMOJI_SENTIMENT = {
    // Happiness/Positivity (0.3 to 0.7)
    '😊': 0.3,
    '😄': 0.4,
    '😁': 0.5,
    '😆': 0.4,
    '😍': 0.6,
    '🥰': 0.7,
    '😻': 0.5,
    '👍': 0.4,
    '🙌': 0.5,
    '✨': 0.3,
    '🎉': 0.6,
    '🎊': 0.6,
    '💕': 0.5,
    '💖': 0.6,
    '🌟': 0.4,
    '💫': 0.3,
    '🌈': 0.5,
    '☀️': 0.4,
    '🥳': 0.7,
    '🤗': 0.6,
    '😇': 0.5,

    // Sadness/Negativity (-0.6 to -0.3)
    '😢': -0.5,
    '😭': -0.6,
    '😞': -0.4,
    '😔': -0.3,
    '💔': -0.6,
    '😡': -0.7,
    '🤬': -0.9,
    '😤': -0.5,
    '😠': -0.6,
    '😖': -0.5,
    '😫': -0.4,
    '😩': -0.4,
    '😪': -0.3,
    '😨': -0.5,
    '😰': -0.6,
    '😳': -0.2,
    '😱': -0.4,
    '😵': -0.4,
    '🤐': 0,
    '😶': -0.2,

    // Neutral/Humor (varies)
    '😐': 0,
    '😒': -0.2,
    '🙄': -0.1,
    '🤡': -0.3,
    '💀': 0,
    '😂': 0.4,
    '🤣': 0.5,
    '😅': 0.2,
    '😬': -0.2,
    '🤔': 0,
    '🧐': 0,
    '👀': 0,
    '💭': 0,
    '🎭': 0,
    '🤷': 0,

    // Thinking/Contemplation (neutral)
    '🙏': 0,
    '🤝': 0,
    '✋': 0,
    '👋': 0,
    '🤲': 0,

    // Urgency/Energy (0.2 to 0.3)
    '🔥': 0.3,
    '⚡': 0.3,
    '💥': 0.2,
    '🚨': -0.4,
    '⚠️': -0.3,
    '🎬': 0.2,
    '🎪': 0.2,
    '🌪️': -0.2,
    '⛈️': -0.3,

    // Love/Connection (0.4 to 0.7)
    '❤️': 0.6,
    '💙': 0.5,
    '💚': 0.5,
    '💛': 0.5,
    '🧡': 0.5,
    '💜': 0.5,
    '🖤': -0.2,
    '🤍': 0.3,
    '🤎': 0.3,

    // Disappointment/Frustration (-0.4 to -0.2)
    '😑': -0.2,
    '😏': -0.2,
    '😌': 0,
    '🤥': -0.3,
    '😾': -0.4,

    // Success/Achievement (0.5 to 0.7)
    '🏆': 0.6,
    '🥇': 0.6,
    '🥈': 0.5,
    '🥉': 0.4,
    '⭐': 0.5,
    '🎖️': 0.6,
    '👑': 0.5,
    '💎': 0.5,

    // Various animals & nature
    '🐶': 0.3,
    '😺': 0.4,
    '😸': 0.4,
    '🐻': 0.2,
    '🐯': -0.1,
    '🐴': 0.1,
    '🐒': 0.3,
    '🦋': 0.4,
    '🌹': 0.3,
    '🌻': 0.4,
    '🌺': 0.3,
    '🍄': 0.1,
    '💐': 0.4,
    '🌿': 0.2,
};

// ============================================================================
// CONTEXTUAL PATTERNS (Complex Rules - Advanced)
// ============================================================================
export const CONTEXTUAL_PATTERNS = {
    // Pattern 1: Fake Positivity (sarcasm masked as positivity during negative discussions)
    fakePositivity: {
        condition: (analysis) => analysis.valence > 0.4 && analysis.sarcasm > 0.6,
        adjustment: { valence: -0.2, sarcasm: 0.1 },
        classification: 'fake_positivity',
        actionGate: { canLike: false, canRetweet: false },
    },

    // Pattern 2: Restrained Grief (sad but controlled - respectful mourning)
    restrainedGrief: {
        condition: (analysis) => analysis.valence < -0.3 && analysis.arousal < 0.3,
        classification: 'healthy_mourning',
        actionGate: {
            canLike: false,
            canReply: false,
            canRetweet: false,
            canQuote: false,
            canExpand: true,
        },
    },

    // Pattern 3: Passionate Advocacy (high dominance + urgency + positive + low toxicity)
    passionateAdvocacy: {
        condition: (analysis) =>
            analysis.dominance > 0.6 &&
            analysis.urgency > 0.6 &&
            analysis.valence > 0.3 &&
            analysis.toxicity < 0.4,
        classification: 'constructive_activism',
        recommendedTone: 'assertive',
        engagementBoost: 0.3,
    },

    // Pattern 4: Toxic Ranting (high arousal + high toxicity + low dominance)
    toxicRanting: {
        condition: (analysis) =>
            analysis.arousal > 0.7 && analysis.toxicity > 0.6 && analysis.dominance < 0.5,
        classification: 'emotional_outburst',
        actionGate: {
            canLike: false,
            canReply: false, // Too risky to engage
            canRetweet: false,
            canQuote: false,
        },
    },

    // Pattern 5: Intellectual Debate (high dominance + low toxicity + specific topic)
    intellectualDebate: {
        condition: (analysis) =>
            analysis.dominance > 0.6 && analysis.toxicity < 0.3 && analysis.arousal < 0.6,
        classification: 'constructive_discussion',
        recommendedTone: 'factual',
        engagementBoost: 0.2,
    },

    // Pattern 6: Sarcastic Commentary (high sarcasm + context-dependent)
    sarcasticCommentary: {
        condition: (analysis) => analysis.sarcasm > 0.7,
        classification: 'witty_observation',
        requiresPersonalityMatch: true, // Only "joker" personality should reply
        recommendedTone: 'witty',
    },

    // Pattern 7: Crisis/Emergency (very high urgency + context specific)
    crisis: {
        condition: (analysis) =>
            analysis.urgency > 0.9 && (analysis.valence < -0.5 || analysis.toxicity > 0.7),
        classification: 'emergency_situation',
        actionGate: {
            canLike: false,
            canRetweet: false, // Don't amplify emergencies
            canQuote: true, // Can add context
            canReply: true, // Can provide help
        },
    },

    // Pattern 8: Celebration/Excitement (high arousal + high valence + low toxicity)
    celebration: {
        condition: (analysis) =>
            analysis.arousal > 0.7 && analysis.valence > 0.6 && analysis.toxicity < 0.2,
        classification: 'joyful_event',
        recommendedTone: 'enthusiastic',
        engagementBoost: 0.5,
    },
};

// ============================================================================
// ACTION GATING RULES (When to engage - Explicit Rules)
// ============================================================================
export const ACTION_GATES = {
    reply: {
        minValence: -0.8, // Can reply to moderately negative
        maxToxicity: 0.7, // But not too toxic
        sarcasmOk: true, // Sarcasm is fine if we match tone
        urgencyThreshold: null, // No urgency limit for replies
        mustNotHavePattern: ['restrainedGrief'],
        description: 'Can engage thoughtfully with most content',
    },

    like: {
        minValence: -0.2, // Only like neutral/positive
        maxToxicity: 0.2, // Must be friendly
        sarcasmOk: false, // Don't like sarcasm (seems sarcastic back)
        exclusiveNegative: ['grief', 'tragedy', 'death', 'suicide'],
        mustNotHavePattern: ['restrainedGrief', 'toxicRanting'],
        description: 'Conservative - only like genuine positive content',
    },

    quote: {
        minValence: -0.3, // Slightly more negative tolerance than like
        maxToxicity: 0.3, // Keep it civil
        sarcasmOk: true, // Can quote sarcasm with own comment
        urgencyThreshold: null,
        canQuoteSarcasm: true,
        mustNotHavePattern: ['restrainedGrief', 'toxicRanting'],
        description: 'Medium risk - can add context to controversial tweets',
    },

    retweet: {
        minValence: -0.1, // Nearly positive only
        maxToxicity: 0.1, // Must be very friendly
        sarcasmOk: false, // Don't amplify sarcasm
        exclusiveNegative: ['grief', 'tragedy', 'death', 'emergency', 'crisis', 'suicide'],
        mustNotHavePattern: ['restrainedGrief', 'toxicRanting', 'fakePositivity'],
        description: "Most conservative - don't amplify dark/controversial content",
    },

    bookmark: {
        minValence: -0.5, // Save things we want to remember/research
        maxToxicity: 0.6, // Can bookmark info even if toxic
        sarcasmOk: true, // Save interesting sarcasm
        urgencyThreshold: null,
        description: 'Personal collection - save interesting/important content',
    },
};

// ============================================================================
// PERSONALITY PROFILES (Different personalities have different engagement preferences)
// ============================================================================
export const PERSONALITY_PROFILES = {
    observer: {
        replyProbability: 0.3,
        preferredTones: ['intellectual', 'analytical'],
        sarcasticTolerance: 0.4,
        toxicityTolerance: 0.2,
        dominancePreference: 0.5, // Medium dominance tweets
        arousalThreshold: 0.5, // Prefers thoughtful, not emotional
        negativeBias: -0.1, // Slightly likes critical/analytical content
        description: 'Quiet observer - thoughtful but selective',
    },

    enthusiast: {
        replyProbability: 0.7,
        preferredTones: ['positive', 'enthusiastic', 'excitable'],
        sarcasticTolerance: 0.3,
        toxicityTolerance: 0.1,
        dominancePreference: 0.6,
        arousalThreshold: 0.7, // Likes high-energy content
        negativeBias: 0.2, // Avoids negative content
        description: 'Energetic participant - loves positive energy',
    },

    analyst: {
        replyProbability: 0.5,
        preferredTones: ['intellectual', 'assertive', 'factual'],
        sarcasticTolerance: 0.6, // Gets sarcasm/humor
        toxicityTolerance: 0.3,
        dominancePreference: 0.7, // Likes assertive takes
        arousalThreshold: 0.4, // Prefers calm, facts-based content
        negativeBias: 0.1, // Can engage with criticism if well-reasoned
        description: 'Thoughtful debater - data-driven and witty',
    },

    joker: {
        replyProbability: 0.8,
        preferredTones: ['witty', 'humorous', 'sarcastic'],
        sarcasticTolerance: 0.9, // Masters at sarcasm
        toxicityTolerance: 0.4, // Can handle edgier content
        dominancePreference: 0.5,
        arousalThreshold: 0.6, // Likes entertaining content
        negativeBias: 0.0, // Neutral on valence
        description: 'Witty comedian - sarcastic and fun',
    },

    advocate: {
        replyProbability: 0.75,
        preferredTones: ['passionate', 'assertive', 'motivating'],
        sarcasticTolerance: 0.2, // No patience for snark
        toxicityTolerance: 0.2,
        dominancePreference: 0.8, // Likes strong opinions
        arousalThreshold: 0.7, // Energized by action-oriented content
        negativeBias: 0.2, // Motivated by identifying problems
        description: 'Passionate activist - driven by purpose',
    },

    empath: {
        replyProbability: 0.4,
        preferredTones: ['supportive', 'empathetic', 'caring'],
        sarcasticTolerance: 0.1, // Sarcasm feels hurtful
        toxicityTolerance: 0.1,
        dominancePreference: 0.2, // Prefers gentle approach
        arousalThreshold: 0.3, // Prefers calm, peaceful content
        negativeBias: -0.3, // Avoids toxic/aggressive content
        description: 'Caring supporter - emotionally intelligent',
    },
};

// ============================================================================
// CONTEXT-AWARE KEYWORDS (Topic-specific sentiment modifiers)
// ============================================================================
export const TOPIC_KEYWORDS = {
    // Political topics (high risk)
    politics: [
        'election',
        'vote',
        'campaign',
        'candidate',
        'senate',
        'congress',
        'president',
        'party',
        'policy',
        'republican',
        'democrat',
        'liberal',
        'conservative',
        'left',
        'right',
        'wing',
        'trump',
        'biden',
        'harris',
    ],

    // Religion (sensitive)
    religion: [
        'god',
        'allah',
        'jesus',
        'jesus christ',
        'christ',
        'god',
        'christian',
        'muslim',
        'jewish',
        'atheist',
        'faith',
        'prayer',
        'church',
        'mosque',
        'temple',
        'scripture',
        'bible',
        'quran',
    ],

    // Social justice (emotionally charged)
    socialJustice: [
        'racism',
        'racist',
        'sexism',
        'sexist',
        'discrimination',
        'equal',
        'rights',
        'justice',
        'equality',
        'bias',
        'privilege',
        'marginalize',
        'intersectionality',
        'oppression',
        'liberation',
        'revolution',
    ],

    // Health/Medical (sensitive, specific rules)
    health: [
        'vaccine',
        'vaccination',
        'covid',
        'pandemic',
        'mental health',
        'depression',
        'anxiety',
        'suicide',
        'cancer',
        'disease',
        'disability',
        'treatment',
        'medicine',
        'doctor',
        'hospital',
        'illness',
    ],

    // Technology/Business
    technology: [
        'ai',
        'startup',
        'tech',
        'software',
        'hardware',
        'app',
        'platform',
        'algorithm',
        'crypto',
        'bitcoin',
        'nft',
        'web3',
        'elon',
        'musk',
    ],

    // Social (relationship-based)
    social: [
        'friend',
        'family',
        'relationship',
        'dating',
        'break up',
        'divorce',
        'couple',
        'marriage',
        'wedding',
        'love',
        'heartbreak',
        'dating app',
    ],
};

// ============================================================================
// Sentiment Score Thresholds (Action triggers)
// ============================================================================
export const SENTIMENT_THRESHOLDS = {
    skipLike: 0.15,
    skipRetweet: 0.08,
    skipReply: 0.25,
    skipQuote: 0.2,
    skipBookmark: 0.5,
    allowExpand: true,

    // Advanced thresholds
    toxicityRedLine: 0.8, // Absolute no-go zone
    griefThreshold: -0.7, // Grief detection
    spamConfidence: 0.85, // Likely bot/spam
    authenticityMin: 0.2, // Too weird/artificial to engage
};

export default {
    POSITIVE_LEXICON,
    NEGATIVE_LEXICON,
    AROUSAL_MARKERS,
    DOMINANCE_MARKERS,
    SARCASM_MARKERS,
    URGENCY_MARKERS,
    TOXICITY_MARKERS,
    EMOJI_SENTIMENT,
    CONTEXTUAL_PATTERNS,
    ACTION_GATES,
    PERSONALITY_PROFILES,
    TOPIC_KEYWORDS,
    SENTIMENT_THRESHOLDS,
};
