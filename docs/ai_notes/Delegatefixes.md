	•	The Only the delegates can withdraw sol not the owner.
	•	Implement a withdrawal request method for delegates only.
	•	Withdraw request must provide how much and what token.
	•	Actual withdrawals can occur after an adjustable timeframe.
	•	Withdrawal wait time can be 5 minutes to 72 hours.
	•	Wait time is set for each individual delegate by contract owner.
	•	Only one active withdrawal request per delegate is allowed.
	•	Prevent additional withdrawal requests while one is pending.
	•	Contract owner can cancel any delegate’s pending withdrawal.
	•	Token swap fees are adjustable by owner up to 0.5%.
	•	Default starting fee at 0% initially.
	•	There is a fixed fee of 0.0000125 SOL per swap transaction.
