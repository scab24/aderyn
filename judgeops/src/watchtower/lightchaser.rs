// LightChaser is one of the flavors that implements WatchTower
// Twitter Handle - https://twitter.com/ChaseTheLight99
//
// Inspired from series on twitter -
//
// Chapter 001 Dynamic Accuracy
// https://twitter.com/ChaseTheLight99/status/1749521023446171722
//
// Chapter 002 Quality Assurance
// https://twitter.com/ChaseTheLight99/status/1749830985884090785
//
// Chapter 003 Dynamic Severity
// https://twitter.com/ChaseTheLight99/status/1750508847947522519
//
// And more...

use std::collections::{hash_map, HashMap};
use std::process::ExitCode;

use strum::EnumCount;

use super::utils::MetricsDatabase;
use super::{
    CalculatesValueOfDetector, GetsCurrentMetricsForDetector, GetsRegisteredDetectors,
    GiveOverallOpinion, InfersMetrics, Metrics, RegistersNewDetector, TakesFeedbackFromJudge,
    WatchTower,
};

use crate::IssueSeverity;

pub struct LightChaser {
    pub metrics_db: MetricsDatabase,
}

impl RegistersNewDetector for LightChaser {
    fn register(&self, detector_name: String, assigned_severity: IssueSeverity) {
        // Chapter 001 - "Every detector is given a accuracy score, this score defaults to 5."
        // Follow up question - Where does 5 come from ?
        // Answered here - https://x.com/ChaseTheLight99/status/1750111766162358288?s=20
        self.metrics_db
            .register_new_detector(detector_name, assigned_severity);
    }
}

impl GetsCurrentMetricsForDetector for LightChaser {
    fn metrics(&self, detector_name: String) -> Metrics {
        self.metrics_db.get_metrics(detector_name)
    }
    fn all_metrics(&self) -> HashMap<String, Metrics> {
        self.metrics_db.get_all_metrics()
    }
}

impl TakesFeedbackFromJudge for LightChaser {
    /// Light Chaser caps the value of TP - FP from 0 to IssueSeverity::COUNT
    /// Refer to Chapter 001 - Link at the top of this page
    fn take_feedback(&self, feedback: super::Feedback) {
        feedback
            .negative_feedbacks
            .iter()
            .for_each(|detector_name| {
                let current_metrics = self.metrics_db.get_metrics(detector_name.clone());
                let current_lc_accuracy = current_metrics.lc_accuracy();

                if current_lc_accuracy >= 1 {
                    self.metrics_db
                        .increase_false_positive_with_trigger_count(detector_name.clone());
                } else {
                    self.metrics_db.increase_trigger_count(detector_name.clone());
                    eprintln!("WARNING: detector {}'s lc_accuracy = 0 ! \nUnable to process Negative Feedback", detector_name);
                }
            });

        feedback
            .positive_feedbacks
            .iter()
            .for_each(|detector_name| {
                let current_metrics = self.metrics_db.get_metrics(detector_name.clone());
                let current_lc_accuracy = current_metrics.lc_accuracy();

                if current_lc_accuracy != 0
                    && current_lc_accuracy < (IssueSeverity::COUNT - 1) as u64
                {
                    // NOTE: LightChaser follows the "Once imperfect, always imperfect" strategy
                    // Therefore, an imperfect score can never "rise" to a perfect score of IssueSeverity::COUNT
                    self.metrics_db
                        .increase_true_positive_with_trigger_count(detector_name.clone());
                } else {
                    self.metrics_db
                        .increase_trigger_count(detector_name.clone());
                }
            });

        feedback
            .all_exposed_detectors
            .into_iter()
            .for_each(|detector_name| self.metrics_db.increase_experience(detector_name.0));
    }
}

impl InfersMetrics for Metrics {
    fn is_acceptable(&self) -> bool {
        //"If a detector is triggered, its accuracy score is looked up and the following cap is applied."
        //      - @ChaseTheLight99, 2024
        // Chapter 003 - Dynamic Severity https://twitter.com/ChaseTheLight99/status/1750508847947522519
        let lc_accuracy = self.lc_accuracy();
        if lc_accuracy == 0 {
            return false;
        }

        match self.current_severity {
            IssueSeverity::High => lc_accuracy >= IssueSeverity::COUNT as u64 - 1,
            IssueSeverity::Medium => lc_accuracy >= IssueSeverity::COUNT as u64 - 2,
            IssueSeverity::Low => lc_accuracy >= IssueSeverity::COUNT as u64 - 3,
            IssueSeverity::NC => lc_accuracy >= IssueSeverity::COUNT as u64 - 4,
        }
    }
}

impl Metrics {
    pub(crate) fn lc_accuracy(&self) -> u64 {
        // NOTE: TP - FP is scientifically not the same as accuracy, however this value is referred as
        // "accuracy" in the lc (light chaser) tweets. So I'm putting that out here "as is".
        self.true_positives - self.false_positives
    }
}

impl GiveOverallOpinion for LightChaser {
    fn give_overall_opinion(&self) -> ExitCode {
        let mut found_suggestion = false;
        println!();
        // Suggest removing very poorly performing core detectors (lc_accuracy == 0)
        self.metrics_db
            .get_all_detectors_names()
            .iter()
            .for_each(|d| {
                let d_metrics = self.metrics_db.get_metrics(d.clone());
                if d_metrics.lc_accuracy() == 0 {
                    println!("{d} should be removed as it's accuracy has fallen to 0 ! ");
                    found_suggestion = true;
                }
            });
        println!();
        // Suggest downgrading poorly performing core detectors (lc_accuracy doesn't live up to its severity)
        self.metrics_db
            .get_all_detectors_names()
            .iter()
            .for_each(|d| {
                let d_metrics = self.metrics_db.get_metrics(d.clone());
                if !d_metrics.is_acceptable() && d_metrics.lc_accuracy() != 0 {
                    // The case where lc_accuracy = 0 has been taken care above (we completely remove them)
                    println!(
                        "{d}'s accuracy of {} is unacceptable for {} severity ! ",
                        d_metrics.lc_accuracy(),
                        d_metrics.current_severity,
                    );
                    found_suggestion = true;
                }
            });
        // Suggest giving more feedbacks for inexperienced detectors
        // Group detectors by experience
        let mut experience_map: HashMap<u64, Vec<String>> = HashMap::new();
        println!();
        for detector_name in self.metrics_db.get_all_detectors_names() {
            let d_metrics = self.metrics_db.get_metrics(detector_name.clone());
            if let hash_map::Entry::Vacant(e) = experience_map.entry(d_metrics.experience) {
                e.insert(vec![detector_name]);
            } else {
                let experienced_detectors = experience_map.get_mut(&d_metrics.experience).unwrap();
                experienced_detectors.push(detector_name);
            }
        }

        let mut experiences = experience_map.keys().collect::<Vec<_>>();
        experiences.sort_by(|&a, &b| b.cmp(a));

        // If all the detectors are equally experienced, we don't have to warn about anything
        // Otherwise, we can print the range of experiences and it's upto the admin what they want to
        // do about it
        if experiences.len() > 1 {
            println!(
                "\nDetectors' experiences range from {} to {}",
                *experiences.last().unwrap(),
                experiences.first().unwrap()
            );

            // TODO define a threshold for the above range

            println!("{: <15}\tNumber of detectors", "Experience");
            for exp in &experiences {
                println!("{: <15}\t{}", exp, experience_map.get(exp).unwrap().len());
            }
            println!("Please submit more feedback for detectors with less experience. More equal, fairer it is !\n");

            for exp in &experiences {
                println!(
                    "Experience {}\n{}\n",
                    exp,
                    experience_map.get(exp).unwrap().join(",")
                );
            }
            found_suggestion = true;
        }

        // Suggest taking a look at not so well performing detectors

        // Group detectors by accuracy
        let mut lc_accuracy_map: HashMap<u64, Vec<String>> = HashMap::new();

        for detector_name in self.metrics_db.get_all_detectors_names() {
            let d_metrics = self.metrics_db.get_metrics(detector_name.clone());
            if let hash_map::Entry::Vacant(e) = lc_accuracy_map.entry(d_metrics.lc_accuracy()) {
                e.insert(vec![detector_name]);
            } else {
                let detector_group = lc_accuracy_map.get_mut(&d_metrics.lc_accuracy()).unwrap();
                detector_group.push(detector_name);
            }
        }

        println!("{: <15}\tNumber of detectors", "Accuracy");
        for acc in &lc_accuracy_map.keys().collect::<Vec<_>>() {
            println!("{: <15}\t{}", acc, lc_accuracy_map.get(acc).unwrap().len());
        }

        for acc in &lc_accuracy_map.keys().collect::<Vec<_>>() {
            println!(
                "Accuracy {}\n{}\n",
                acc,
                lc_accuracy_map.get(acc).unwrap().join(",")
            );
        }

        if !found_suggestion {
            println!("No suggestions found. You are good to go!");
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

impl CalculatesValueOfDetector for LightChaser {
    // "Detector Value = Severity * Accuracy * Trigger rate" - ChaseTheLight99
    // https://twitter.com/ChaseTheLight99/status/1745840813685223917
    // Trigger Rate defined here - https://x.com/ChaseTheLight99/status/1750202276017283179?s=20
    // NOTE - Here, the formula is slightly modified (we're not taking severity into account) because
    // we would be grouping detectors by IssueSeverity so it doesn't become a factor
    // Value to assign to severity is upto whoever is using this API
    fn value(&self, detector_name: String) -> f64 {
        let metrics = self.metrics_db.get_metrics(detector_name);
        self.value_from_metrics(&metrics)
    }

    #[allow(clippy::let_and_return)]
    fn value_from_metrics(&self, metrics: &Metrics) -> f64 {
        let trigger_rate = metrics.trigger_count as f64 / metrics.experience as f64;
        let lc_accuracy = metrics.lc_accuracy();
        let detector_value = trigger_rate * lc_accuracy as f64; // min value = 0, max value = 1 * 5 = 5
        let normalized_value = detector_value / IssueSeverity::COUNT as f64; // make it 0 to 1
        normalized_value
    }
}

impl GetsRegisteredDetectors for LightChaser {
    fn get_registered_detectors_names(&self) -> Vec<String> {
        self.metrics_db.get_all_detectors_names()
    }
}

impl WatchTower for LightChaser {}

/*

Unit Tests

*/

#[cfg(test)]
mod lightchaser_tests {
    use std::collections::HashMap;

    use serial_test::serial;
    use strum::{Display, EnumCount, EnumString};

    use crate::{
        watchtower::{utils::MetricsDatabase, Feedback, InfersMetrics, WatchTower},
        IssueSeverity,
    };

    use super::LightChaser;

    #[derive(Debug, PartialEq, EnumString, Display)]
    #[strum(serialize_all = "kebab-case")]
    pub(crate) enum IssueDetectorNamePool {
        DelegateCallInLoop,
        CentralizationRisk,
        SolmateSafeTransferLib,
        AvoidAbiEncodePacked,
        Ecrecover,
        DeprecatedOzFunctions,
        UnsafeERC20Functions,
        UnspecificSolidityPragma,
        ZeroAddressCheck,
        UselessPublicFunction,
        ConstantsInsteadOfLiterals,
        UnindexedEvents,
        RequireWithString,
        NonReentrantBeforeOthers,
        BlockTimestampDeadline,
        UnsafeOzERC721Mint,
        PushZeroOpcode,
        ArbitraryTransferFrom,
        // NOTE: `Undecided` will be the default name (for new bots).
        // If it's accepted, a new variant will be added to this enum before normalizing it in aderyn
        Undecided,
    }

    #[test]
    #[serial]
    fn lightchaser_can_register_new_detector() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        let prev_metrics = watchtower.all_metrics();

        assert!(
            !prev_metrics.contains_key(&IssueDetectorNamePool::ArbitraryTransferFrom.to_string())
        );
        assert!(prev_metrics.is_empty());

        let expected_severity = IssueSeverity::High;

        watchtower.register(
            IssueDetectorNamePool::ArbitraryTransferFrom.to_string(),
            expected_severity.clone(),
        );

        let after_metrics = watchtower.all_metrics();
        assert!(
            after_metrics.contains_key(&IssueDetectorNamePool::ArbitraryTransferFrom.to_string())
        );

        let after_metrics_arbitrary = after_metrics
            .get(&IssueDetectorNamePool::ArbitraryTransferFrom.to_string())
            .unwrap();
        assert!(after_metrics_arbitrary.current_severity == expected_severity);
    }

    #[test]
    #[serial]
    fn lightchaser_assigns_perfect_score_by_default() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::ArbitraryTransferFrom.to_string(),
            IssueSeverity::High,
        );

        let arbitrary_metrics =
            watchtower.metrics(IssueDetectorNamePool::ArbitraryTransferFrom.to_string());

        // New detectors should get a perfect accuracy be default.
        assert!(arbitrary_metrics.lc_accuracy() == IssueSeverity::COUNT as u64);
    }

    #[test]
    #[serial]
    fn lightchaser_doesnt_accept_medium_detectors_with_lc_accuracy_less_than_3() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 3);
        assert!(current_metrics.is_acceptable());

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 2);
        assert!(current_metrics.is_acceptable());

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 1);
        assert!(!current_metrics.is_acceptable());
    }

    #[test]
    #[serial]
    fn lightchaser_decreases_detector_value_if_it_doesnt_trigger() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        let before_value = watchtower.value(IssueDetectorNamePool::CentralizationRisk.to_string());

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let after_value = watchtower.value(IssueDetectorNamePool::CentralizationRisk.to_string());

        assert!(after_value < before_value);
    }

    #[test]
    #[serial]
    fn lightchaser_lc_accuracy_can_increase_after_it_decreases() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 3);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 2);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 1);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 2);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 3);
    }

    #[test]
    #[serial]
    fn lightchaser_accuracy_doesnt_increase_beyond_5() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 4);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 4);
    }

    #[test]
    #[serial]
    fn lightchaser_accuracy_cannot_become_perfect_once_imperfect() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 4);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![],
            negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 3);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 3);
    }

    #[test]
    #[serial]
    fn lightchaser_accuracy_cannot_recover_once_it_is_0() {
        let test_metrics_db =
            MetricsDatabase::from_path("lightchaser_tests.metrics_db.json".to_string());

        test_metrics_db.self_delete();
        test_metrics_db.create_if_not_exists();

        let watchtower: Box<dyn WatchTower> = Box::new(LightChaser {
            metrics_db: test_metrics_db,
        });

        watchtower.register(
            IssueDetectorNamePool::CentralizationRisk.to_string(),
            IssueSeverity::Medium,
        );

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 4);

        for _ in 1..=5 {
            watchtower.take_feedback(Feedback {
                positive_feedbacks: vec![],
                negative_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
                all_exposed_detectors: HashMap::from([(
                    IssueDetectorNamePool::CentralizationRisk.to_string(),
                    IssueSeverity::Medium.to_string(),
                )]),
            });
        }

        let current_metrics =
            watchtower.metrics(IssueDetectorNamePool::CentralizationRisk.to_string());
        assert!(current_metrics.lc_accuracy() == 0);

        watchtower.take_feedback(Feedback {
            positive_feedbacks: vec![IssueDetectorNamePool::CentralizationRisk.to_string()],
            negative_feedbacks: vec![],
            all_exposed_detectors: HashMap::from([(
                IssueDetectorNamePool::CentralizationRisk.to_string(),
                IssueSeverity::Medium.to_string(),
            )]),
        });
        assert!(current_metrics.lc_accuracy() == 0);
    }
}
