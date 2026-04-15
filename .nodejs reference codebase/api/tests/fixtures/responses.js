/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

// Mock API responses for testing

// Sample successful Ollama response
export const mockOllamaResponse = {
    done: true,
    response: 'Test response from AI',
};

// Sample Ollama streaming response chunk
export const mockOllamaStreamChunk = {
    done: false,
    response: 'Test response chunk ',
};

// Sample Ollama streaming final chunk
export const mockOllamaStreamFinal = {
    done: true,
    response: 'Final response',
};

// Sample Twitter tweet data
export const mockTweet = {
    id: '123456789',
    id_str: '123456789',
    text: 'Hello world! This is a test tweet.',
    full_text: 'Hello world! This is a test tweet.',
    created_at: '2024-01-15T12:00:00.000Z',
    author: {
        id: 987654321,
        id_str: '987654321',
        screen_name: 'testuser',
        name: 'Test User',
        profile_image_url_https: 'https://example.com/avatar.jpg',
        followers_count: 100,
        following_count: 50,
    },
    retweet_count: 10,
    favorite_count: 25,
    reply_count: 5,
    quote_count: 2,
    lang: 'en',
    possibly_sensitive: false,
    in_reply_to_status_id_str: null,
    is_quote_status: false,
    retweeted: false,
    favorited: false,
};

// Sample Twitter user data
export const mockTwitterUser = {
    id: 987654321,
    id_str: '987654321',
    screen_name: 'testuser',
    name: 'Test User',
    description: 'This is a test user',
    followers_count: 100,
    friends_count: 50,
    statuses_count: 200,
    created_at: '2020-01-01T00:00:00.000Z',
    profile_image_url_https: 'https://example.com/avatar.jpg',
    profile_banner_url: 'https://example.com/banner.jpg',
    verified: false,
    protected: false,
};

// Sample OpenRouter API response
export const mockOpenRouterResponse = {
    id: 'gen-123',
    choices: [
        {
            index: 0,
            message: {
                role: 'assistant',
                content: 'Test response from OpenRouter',
            },
            finish_reason: 'stop',
        },
    ],
    usage: {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    },
};

// Sample error responses
export const mockErrorResponse = {
    error: {
        message: 'Something went wrong',
        type: 'api_error',
        code: 500,
    },
};

// Sample HTTP response for network failures
export const mockNetworkError = new Error('Network request failed');

// Sample validation error
export const mockValidationError = {
    error: 'Validation failed',
    details: ['Field "test" is required'],
};
