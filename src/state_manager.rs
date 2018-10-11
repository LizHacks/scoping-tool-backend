use mpsc::Receiver;
use std::collections::HashMap;
use std::thread;

use events::*;
use state::*;

pub fn start_state_manager(events_incoming_recv: Receiver<SystemEvents>) {
    let mut session_state = HashMap::new();
    thread::spawn(move || loop {
        match events_incoming_recv.recv().unwrap() {
            SystemEvents::StartNewSessionEvent(e) => {
                session_state.insert(
                    e.session_id,
                    SessionState::new(e.session_title, e.session_description),
                );
            }
            SystemEvents::EndSessionEvent(e) => {
                match session_state.clone().get(&e.session_id) {
                    Some(s) => {
                        let next_state = SessionState {
                            accepting_new_submissions: false,
                            ..(*s).clone()
                        };
                        session_state.insert(e.session_id, next_state.clone());
                        println!("[info] next state: {:?}", next_state);
                    }
                    None => {
                        println!("[warn] invalid id");
                    }
                };
            }
            SystemEvents::SubmitResponseEvent(e) => {
                match session_state.clone().get(&e.session_id) {
                    Some(s) => {
                        let prev_state: SessionState = s.clone();
                        if prev_state.accepting_new_submissions {
                            let mut session_events = prev_state.session_events.clone();
                            session_events.push(SystemEvents::SubmitResponseEvent(e.clone()));
                            let next_state = SessionState {
                                session_events,
                                ..prev_state
                            };

                            session_state.insert(e.session_id, next_state.clone());
                            println!("[info] next state: {:?}", next_state);
                        }
                    }
                    None => {
                        println!("[warn] invalid id");
                    }
                }
            }
            SystemEvents::GetSessionDetails(e) => {
                e.responder
                    .send(match session_state.clone().get(&e.session_id) {
                        Some(state) => Ok(GetSessionDetailsResponse {
                            title: state.title.clone(),
                            description: state.description.clone(),
                        }),
                        None => Err(()),
                    }).unwrap();
            }
            SystemEvents::GetResponseCount(e) => {
                e.responder
                    .send(match session_state.clone().get(&e.session_id) {
                        Some(state) => {
                            let names = state
                                .session_events
                                .iter()
                                .filter_map(|ev| match ev {
                                    SystemEvents::SubmitResponseEvent(submission) => {
                                        Some(submission.name.clone())
                                    }
                                    _ => None,
                                }).collect();
                            Ok(names)
                        }
                        None => Err(()),
                    }).unwrap();
            }
            SystemEvents::GetSessionResult(e) => {
                e.responder
                    .send(match session_state.clone().get(&e.session_id) {
                        Some(state) => {
                            let responses: Vec<SubmissionContent> = state
                                .session_events
                                .iter()
                                .filter_map(|ev| match ev {
                                    SystemEvents::SubmitResponseEvent(submission) => {
                                        Some(SubmissionContent {
                                            name: submission.name.clone(),
                                            value: submission.value.clone(),
                                        })
                                    }
                                    _ => None,
                                }).collect();

                            let average_response = match responses.get(0) {
                                Some(initial_value) => {
                                    responses.iter().fold(initial_value.value, |acc, response| {
                                        (acc + response.value) / 2
                                    })
                                }
                                None => 0,
                            };

                            Ok(GetSessionResultResponse {
                                title: state.title.clone(),
                                description: state.description.clone(),
                                response_count: responses.len(),
                                responses: responses,
                                average_response,
                            })
                        }
                        None => Err(()),
                    }).unwrap();
            }
            _ => {
                println!("System events");
            }
        }
    });
}
