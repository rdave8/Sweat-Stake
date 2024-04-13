pragma solidity ^0.8.0;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";

contract SweatStake {
    IRiscZeroVerifier public immutable verifier;

    bytes32 public constant imageId = ImageID.IS_ENOUGH_ID;

    struct BlockCommitment {
        bytes32 blockHash;
        uint256 blockNumber;
    }

    mapping(address => Goal[]) public addressToGoals;

    struct Goal {
        uint256 startDate;
        uint256 endDate;
        uint256 stakedPerDay;
        uint256 goalPerDay;
        bool[] claimed;
    }

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
    }

    event GoalUpdated(address indexed user, Goal[] updatedGoals);

    function addGoal(uint256 startDate, uint256 endDate, uint256 goalPerDay) public payable {
        require(startDate > block.timestamp, "Start date must be in the future");
        require(endDate > block.timestamp, "End date must be in the future");
        uint256 duration = (endDate - startDate) / 1 days;
        require(duration > 0, "Duration must be at least one day");
        uint256 stakedPerDay = msg.value / duration;

        Goal memory newGoal;
        newGoal.startDate = startDate;
        newGoal.endDate = endDate;
        newGoal.stakedPerDay = stakedPerDay;
        newGoal.goalPerDay = goalPerDay;
        newGoal.claimed = new bool[](duration);

        addressToGoals[msg.sender].push(newGoal);

        emit GoalUpdated(msg.sender, addressToGoals[msg.sender]);
    }

    function getGoals() public view returns(Goal[] memory) {
        return addressToGoals[msg.sender];
    }

    function goalPerDayOf(address account, uint256 goalIndex) external view returns (uint256) {
        return addressToGoals[account][goalIndex].goalPerDay;
    }

    function deleteGoal(uint256 index) public {
        require(index < addressToGoals[msg.sender].length, "Index out of bounds");

        for (uint i = index; i < addressToGoals[msg.sender].length - 1; i++) {
            addressToGoals[msg.sender][i] = addressToGoals[msg.sender][i + 1];
        }

        addressToGoals[msg.sender].pop();

        emit GoalUpdated(msg.sender, addressToGoals[msg.sender]);
    }

    function claim(bytes calldata journal, bytes32 postStateDigest, bytes calldata seal, uint256 goalIndex) public {
        Goal storage goal = addressToGoals[msg.sender][goalIndex];
        require(goal.endDate >= block.timestamp, "Goal has ended");

        uint256 dayIndex = (block.timestamp - goal.startDate) / 1 days;
        require(dayIndex < goal.claimed.length, "Day index out of bounds");
        require(!goal.claimed[dayIndex], "Already claimed for this day");

        BlockCommitment memory commitment = abi.decode(journal, (BlockCommitment));
        require(blockhash(commitment.blockNumber) == commitment.blockHash);
        require(verifier.verify(seal, imageId, postStateDigest, sha256(journal)));

        goal.claimed[dayIndex] = true;
        payable(msg.sender).transfer(goal.stakedPerDay);

        emit GoalUpdated(msg.sender, addressToGoals[msg.sender]);
    }
}
