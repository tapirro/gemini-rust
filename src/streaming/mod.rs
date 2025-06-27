//! Streaming support for Gemini API responses

use crate::{
    error::{Error, Result},
    models::GenerateContentResponse,
};
use futures::{Stream, StreamExt as FuturesStreamExt};
use reqwest::Response;
use std::pin::Pin;

/// Parse a streaming response into a stream of results
pub fn parse_stream(response: Response) -> impl Stream<Item = Result<GenerateContentResponse>> {
    let stream = response.bytes_stream();

    futures::stream::unfold(
        (stream, Vec::new()),
        |(mut stream, mut buffer)| async move {
            loop {
                match FuturesStreamExt::next(&mut stream).await {
                    Some(Ok(chunk)) => {
                        buffer.extend_from_slice(&chunk);

                        // Try to parse complete JSON objects from buffer
                        if let Some((result, remaining)) = try_parse_json(&buffer) {
                            buffer = remaining;
                            return Some((result, (stream, buffer)));
                        }
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(Error::Streaming(format!("Stream error: {}", e))),
                            (stream, buffer),
                        ));
                    }
                    None => {
                        // Stream ended, try to parse any remaining data
                        if !buffer.is_empty() {
                            if let Ok(response) = serde_json::from_slice(&buffer) {
                                return Some((Ok(response), (stream, Vec::new())));
                            }
                        }
                        return None;
                    }
                }
            }
        },
    )
}

/// Try to parse a complete JSON object from the buffer
fn try_parse_json(buffer: &[u8]) -> Option<(Result<GenerateContentResponse>, Vec<u8>)> {
    // Look for complete JSON objects by counting braces
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut json_end = None;

    for (i, &byte) in buffer.iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match byte {
            b'"' if !in_string => in_string = true,
            b'"' if in_string => in_string = false,
            b'\\' if in_string => escape_next = true,
            b'{' if !in_string => brace_count += 1,
            b'}' if !in_string => {
                brace_count -= 1;
                if brace_count == 0 && i > 0 {
                    json_end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    if let Some(end) = json_end {
        let json_bytes = &buffer[..end];
        let remaining = buffer[end..].to_vec();

        match serde_json::from_slice(json_bytes) {
            Ok(response) => Some((Ok(response), remaining)),
            Err(e) => Some((Err(Error::Json(e)), remaining)),
        }
    } else {
        None
    }
}

/// Stream processor that accumulates partial responses
pub struct StreamAccumulator {
    accumulated_text: String,
    current_response: Option<GenerateContentResponse>,
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamAccumulator {
    /// Create a new stream accumulator
    pub fn new() -> Self {
        Self {
            accumulated_text: String::new(),
            current_response: None,
        }
    }

    /// Process a streaming response chunk
    pub fn process_chunk(&mut self, response: GenerateContentResponse) -> Option<String> {
        // Extract text from the response
        let text = response.candidates.first().and_then(|candidate| {
            candidate.content.parts.first().and_then(|part| {
                if let crate::models::Part::Text { text } = part {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });

        if let Some(ref text) = text {
            self.accumulated_text.push_str(text);
        }

        self.current_response = Some(response);
        text
    }

    /// Get the complete accumulated text
    pub fn get_accumulated_text(&self) -> &str {
        &self.accumulated_text
    }

    /// Get the final response with complete text
    pub fn finalize(mut self) -> Option<GenerateContentResponse> {
        if let Some(mut response) = self.current_response.take() {
            // Update the response with the complete accumulated text
            if let Some(candidate) = response.candidates.first_mut() {
                if let Some(crate::models::Part::Text { text }) =
                    candidate.content.parts.first_mut()
                {
                    *text = self.accumulated_text;
                }
            }
            Some(response)
        } else {
            None
        }
    }
}

/// Extension trait for working with streaming responses
pub trait GeminiStreamExt: Stream {
    /// Accumulate streaming text responses into complete text
    fn accumulate_text(self) -> Pin<Box<dyn Stream<Item = Result<String>>>>
    where
        Self: Sized + 'static,
        Self::Item: Into<Result<GenerateContentResponse>>,
    {
        Box::pin(FuturesStreamExt::filter_map(self, |item| async move {
            match item.into() {
                Ok(response) => response.candidates.first().and_then(|candidate| {
                    candidate.content.parts.first().and_then(|part| {
                        if let crate::models::Part::Text { text } = part {
                            Some(Ok(text.clone()))
                        } else {
                            None
                        }
                    })
                }),
                Err(e) => Some(Err(e)),
            }
        }))
    }
}

impl<T> GeminiStreamExt for T where T: Stream {}
