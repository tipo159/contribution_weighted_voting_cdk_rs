use candid::{CandidType, Principal};
use chrono::DateTime;
use ic_cdk::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;

// MAX_POLLS is set to 3 to facilitate testing.
const MAX_POLLS: usize = 3;

#[allow(non_snake_case)]
enum PollError {
    TooManyPolls,
    InvalidDate,
    PollClosingTimeMustFuture,
    PollInUse,
    PollNotExist,
    VoterInUse,
    VoterPrincipalInUse,
    VoterNotExist,
    VoterNotAuthorized,
    CallerNotPollOwner,
    PollOwnerCannotChangeContribution,
    OptionNotExist,
    VotingIsOver,
    OnlyVoterAndPollOwnerCanViewResults,
    VotingNotClosed,
}

impl fmt::Display for PollError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PollError::TooManyPolls => write!(f, "Too many polls created."),
            PollError::InvalidDate => write!(f, "Date format is invalid."),
            PollError::PollClosingTimeMustFuture => {
                write!(f, "Poll closing time must be in the future.")
            }
            PollError::PollInUse => write!(f, "Poll already in use."),
            PollError::PollNotExist => write!(f, "Poll does not exist."),
            PollError::VoterInUse => write!(f, "Voter already in use."),
            PollError::VoterPrincipalInUse => write!(f, "Voter principal already in use."),
            PollError::VoterNotExist => write!(f, "Voter does not exist."),
            PollError::VoterNotAuthorized => {
                write!(f, "Voter is not authorized.")
            }
            PollError::CallerNotPollOwner => write!(f, "Caller is not the poll owner."),
            PollError::PollOwnerCannotChangeContribution => {
                write!(f, "Poll owner cannot change own contribution.")
            }
            PollError::OptionNotExist => write!(f, "Option does not exist."),
            PollError::VotingIsOver => write!(f, "Voting is over."),
            PollError::OnlyVoterAndPollOwnerCanViewResults => {
                write!(
                    f,
                    "Only the voter and the poll owner can view voting results."
                )
            }
            PollError::VotingNotClosed => write!(f, "Voting is not closed."),
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct Poll {
    name: String,
    owner: Principal,
    description: String,
    options: Vec<String>,
    pollClosingDate: String,
    voters: Vec<Voter>,
    votingDetails: Vec<VotingDetail>,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
struct Voter {
    name: String,
    voter: Principal,
    contribution: f32,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
struct VotingDetail {
    name: String,
    option: i32,
    contribution: f32,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct PollPayload {
    name: String,
    description: String,
    options: Vec<String>,
    pollClosingDate: String,
}

type PollResult = Result<Poll, String>;

type VoterResult = Result<Voter, String>;

type VotingDetailResult = Result<VotingDetail, String>;

type VotingResult = Result<Vec<String>, String>;

type PollStore = BTreeMap<String, Poll>;

thread_local! {
    static POLLS: RefCell<PollStore> = RefCell::new(BTreeMap::new());
}

#[allow(clippy::needless_return, non_snake_case)]
#[update]
fn createPoll(payload: PollPayload) -> PollResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let mut polls = polls.borrow_mut();
        if polls.len() == MAX_POLLS {
            return Err(PollError::TooManyPolls.to_string());
        }

        match DateTime::parse_from_rfc3339(&payload.pollClosingDate) {
            Ok(dt) => {
                let poll_closing_at = dt.timestamp_millis() as u64 * 1_000_000;
                if poll_closing_at <= ic_cdk::api::time() {
                    return Err(PollError::PollClosingTimeMustFuture.to_string());
                }
            }
            Err(_) => return Err(PollError::InvalidDate.to_string()),
        }

        if polls.contains_key(&payload.name) {
            return Err(PollError::PollInUse.to_string());
        }

        let poll: Poll = Poll {
            name: payload.name.to_string(),
            owner: caller_principal_id,
            description: payload.description,
            options: payload.options,
            pollClosingDate: payload.pollClosingDate,
            voters: [].to_vec(),
            votingDetails: [].to_vec(),
        };
        polls.insert(payload.name.to_string(), poll.clone());
        return Ok(poll);
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[query]
fn getPollByName(name: String) -> PollResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        if let Some(poll) = polls.borrow().get(&name) {
            let mut poll = poll.clone();
            if poll.owner.to_string() != caller_principal_id.to_string() {
                poll.voters.clear();
                poll.votingDetails.clear();
            }
            return Ok(poll);
        } else {
            return Err(PollError::PollNotExist.to_string());
        }
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[query]
fn getAllPolls() -> Vec<Poll> {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let mut result = polls.borrow().values().cloned().collect::<Vec<Poll>>();
        for value in &mut result {
            if value.owner.to_string() != caller_principal_id.to_string() {
                value.voters.clear();
                value.votingDetails.clear();
            }
        }
        return result;
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[update]
fn registerVoterToPoll(pollname: String, votername: String) -> VoterResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let mut polls = polls.borrow_mut();
        if let Some(poll) = polls.get(&pollname) {
            let mut poll = poll.clone();

            let mut votername_found = false;
            let mut principal_found = false;
            for voter in poll.voters.iter() {
                if voter.name == votername {
                    votername_found = true;
                }
                if voter.voter.to_string() == caller_principal_id.to_string() {
                    principal_found = true;
                }
            }
            if votername_found {
                return Err(PollError::VoterInUse.to_string());
            }
            if principal_found {
                return Err(PollError::VoterPrincipalInUse.to_string());
            }

            let voter = Voter {
                name: votername,
                voter: caller_principal_id,
                contribution: 1.0,
            };
            poll.voters.push(voter.clone());
            polls.insert(pollname, poll);
            return Ok(voter);
        } else {
            return Err(PollError::PollNotExist.to_string());
        }
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[update]
fn changeVoterContribution(pollname: String, votername: String, contribution: f32) -> VoterResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let mut polls = polls.borrow_mut();
        if let Some(poll) = polls.get(&pollname) {
            let mut poll = poll.clone();
            if poll.owner.to_string() != caller_principal_id.to_string() {
                return Err(PollError::CallerNotPollOwner.to_string());
            }

            let index = poll
                .voters
                .iter()
                .position(|v| v.name == votername)
                .ok_or(PollError::VoterNotExist.to_string())?;
            if poll.voters[index].voter.to_string() == caller_principal_id.to_string() {
                return Err(PollError::PollOwnerCannotChangeContribution.to_string());
            }
            poll.voters[index].contribution = contribution;

            polls.insert(pollname, poll.clone());
            return Ok(poll.voters[index].clone());
        } else {
            return Err(PollError::PollNotExist.to_string());
        }
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[update]
fn voteToPoll(pollname: String, votername: String, option: String) -> VotingDetailResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let mut polls = polls.borrow_mut();
        if let Some(poll) = polls.get(&pollname) {
            let mut poll = poll.clone();
            // Confirmed that parse_from_rfc3339 succeeds in createPoll
            let dt = DateTime::parse_from_rfc3339(&poll.pollClosingDate).unwrap();
            let poll_closing_at = dt.timestamp_millis() as u64 * 1_000_000;
            if poll_closing_at <= ic_cdk::api::time() {
                return Err(PollError::VotingIsOver.to_string());
            }

            let index = poll
                .voters
                .iter()
                .position(|v| v.name == votername)
                .ok_or(PollError::VoterNotExist.to_string())?;
            if poll.voters[index].voter.to_string() != caller_principal_id.to_string() {
                return Err(PollError::VoterNotAuthorized.to_string());
            }
            let voter = poll.voters[index].clone();

            let index = poll
                .options
                .iter()
                .position(|o| *o == option)
                .ok_or(PollError::OptionNotExist.to_string())?;

            let votingDetails = VotingDetail {
                name: votername,
                option: index as i32,
                contribution: voter.contribution,
            };
            poll.votingDetails.push(votingDetails.clone());
            polls.insert(pollname, poll);
            return Ok(votingDetails);
        } else {
            return Err(PollError::PollNotExist.to_string());
        }
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[query]
fn getVotingResult(name: String) -> VotingResult {
    let caller_principal_id = ic_cdk::caller();
    return POLLS.with(|polls| {
        let polls = polls.borrow();
        if let Some(poll) = polls.get(&name) {
            // Confirmed that parse_from_rfc3339 succeeds in createPoll
            let dt = DateTime::parse_from_rfc3339(&poll.pollClosingDate).unwrap();
            let poll_closing_at = dt.timestamp_millis() as u64 * 1_000_000;
            if ic_cdk::api::time() < poll_closing_at {
                return Err(PollError::VotingNotClosed.to_string());
            }

            let mut principal_found = false;
            for voter in poll.voters.iter() {
                if voter.voter.to_string() == caller_principal_id.to_string() {
                    principal_found = true;
                }
            }
            if !principal_found && poll.owner.to_string() != caller_principal_id.to_string() {
                return Err(PollError::OnlyVoterAndPollOwnerCanViewResults.to_string());
            }

            let mut no_of_votes: Vec<f32> = vec![0.0; poll.options.len()];
            ic_cdk::print("poll.votingDetails.len().to_string()");
            ic_cdk::print(poll.votingDetails.len().to_string());
            for index in 0..poll.votingDetails.len() {
                no_of_votes[poll.votingDetails[index].option as usize] +=
                    poll.votingDetails[index].contribution;
            }
            let mut results: Vec<String> = vec!["".to_string(); poll.options.len()];
            for index in 0..poll.options.len() {
                results[index] = format!("{}: {:.2}", poll.options[index], no_of_votes[index])
            }
            return Ok(results);
        } else {
            return Err(PollError::PollNotExist.to_string());
        }
    });
}

#[allow(clippy::needless_return, non_snake_case)]
#[update]
fn removeExpiredPolls(over_time: i32) -> Vec<Poll> {
    return POLLS.with(|polls| {
        let mut polls = polls.borrow_mut();
        let mut removed_polls: Vec<Poll> = Vec::new();
        let polls_copy = polls.clone();
        for poll in polls_copy.values() {
            // Confirmed that parse_from_rfc3339 succeeds in createPoll
            let dt = DateTime::parse_from_rfc3339(&poll.pollClosingDate).unwrap();
            let poll_closing_at = dt.timestamp_millis() as u64 * 1_000_000;
            if (poll_closing_at + over_time as u64 * 1_000_000_000) < ic_cdk::api::time() {
                polls.remove(&poll.name);
                removed_polls.push(poll.clone());
            }
        }
        return removed_polls;
    });
}
