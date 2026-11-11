INSERT INTO schema_migrations(version,name,checksum,applied_at)
VALUES (5,'v2_initial_schema','0000000000000000000000000000000000000000000000000000000000000000','2026-01-01T00:00:00Z');

INSERT INTO app_metadata(id,schema_version,database_id,app_version,exam_config_version,created_at,updated_at)
VALUES (1,5,'00000000000000000000000000000000','2.0.0',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO settings(id,preparation_start_date,daily_study_target_minutes,created_at,updated_at)
VALUES (1,NULL,120,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO timer_settings(id,focus_minutes,short_break_minutes,long_break_minutes,notifications_enabled,created_at,updated_at)
VALUES (1,25,5,15,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO revision_settings(id,created_at,updated_at)
VALUES (1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO backup_settings(id,updated_at)
VALUES (1,'2026-01-01T00:00:00Z');

INSERT INTO exams(id,code,name,short_name,authority,official_url,display_order,created_at,updated_at) VALUES
(1,'csir-net-physics','CSIR-UGC NET Physical Sciences','CSIR-NET','CSIR-HRDG / NTA','https://csirnet.nta.nic.in/',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(2,'gate-physics','GATE Physics (PH)','GATE PH','IIT','https://gate.iitm.ac.in/',2,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(3,'tifr-gs-physics','TIFR Graduate School Physics','TIFR GS','TIFR','https://www.tifr.res.in/academics/',3,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(4,'jest-physics','JEST Physics','JEST','JEST Consortium','https://www.jest.org.in/',4,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO exam_profiles(exam_id,is_active,exam_date,created_at,updated_at) VALUES
(1,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(2,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(3,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(4,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO mastery_algorithm_versions(id,version,parameters_json,description,is_active,created_at)
VALUES (1,'1.0','{"studyCap":10,"revisionCap":10,"pyqCap":10,"mockCap":8,"totalPositiveCap":20,"totalNegativeCap":15,"studyPerMinuteDivisor":15,"studyMaxBase":8,"revisionDeltas":{"again":-6,"difficult":2,"good":5,"easy":8},"pyqCorrect":4,"pyqIncorrect":-3,"mockCorrect":3,"mockIncorrect":-2,"difficultyMultipliers":{"1":0.75,"2":0.75,"3":1.0,"4":1.25,"5":1.25}}','initial mastery algorithm',1,'2026-01-01T00:00:00Z');

INSERT INTO readiness_formula_versions(id,version,parameters_json,description,is_active,created_at)
VALUES (1,'1.0','{"weights":{"mastery":0.35,"coverage":0.25,"pyq":0.20,"mock":0.15,"revision":0.05},"minSamples":{"pyq":10,"mock":1,"revision":5},"recencyDays":{"pyq":365,"mock":365,"revision":30}}','initial readiness formula',1,'2026-01-01T00:00:00Z');