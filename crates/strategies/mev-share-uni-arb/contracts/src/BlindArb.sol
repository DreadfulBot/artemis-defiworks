// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Owned} from "solmate/auth/Owned.sol";

interface IERC20 {
    function deposit() external payable;
    function withdraw(uint256) external;
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
}

library SafeMath {
    function add(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require((z = x + y) >= x, "ds-math-add-overflow");
    }

    function sub(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require((z = x - y) <= x, "ds-math-sub-underflow");
    }

    function mul(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
    }
}

interface IUniswapV2Pair {
    function token0() external view returns (address);

    function token1() external view returns (address);
    function getReserves()
        external
        view
        returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function swap(
        uint256 amount0Out,
        uint256 amount1Out,
        address to,
        bytes calldata data
    ) external;
}

interface IUniswapV3Pool {
    function swap(
        address recipient,
        bool zeroForOne,
        int256 amountSpecified,
        uint160 sqrtPriceLimitX96,
        bytes calldata data
    ) external returns (int256 amount0, int256 amount1);
}

interface IUniswapV3SwapCallback {
    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata data
    ) external;
}
interface IUniswapV2Factory {
    function getPair(
        address tokenA,
        address tokenB
    ) external view returns (address pair);
}

contract BlindArb is Owned, IUniswapV3SwapCallback {
    using SafeMath for uint256;
    IERC20 internal constant WETH =
        IERC20(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

    uint160 internal constant MIN_SQRT_RATIO = 4295128739;
    uint160 internal constant MAX_SQRT_RATIO =
        1461446703485210103287273052203988822378723970342;

    IUniswapV2Factory factoryV2 =
        IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);

    address callBackAddress = address(0);
    address nextAddress = address(0);
    bool token0 = true;

    constructor() Owned(msg.sender) {}

    function executeArb__WETH_token0(
        address v2Pair,
        address v3Pair,
        uint256 amountIn,
        uint256 percentageToPayToCoinbase
    ) public onlyOwner {
        // this is header taken from token1 function

        // uint256 gasStart = gasleft();
        // callBackAddress = v3Pair;
        // nextAddress = v2Pair;
        // token0 = false;

        uint256 gasStart = gasleft();
        callBackAddress = v3Pair;

        // is this correct v2Pair address here?
        nextAddress = v2Pair;

        uint256 balanceBefore = WETH.balanceOf(address(this));

        // Swap on V3
        IUniswapV3Pool(v3Pair).swap(
            // also reverted this line, and now test passes
            // address(this),
            v2Pair,
            true,
            int256(amountIn),
            MIN_SQRT_RATIO + 1,
            ""
        );

        uint256 balanceAfter = WETH.balanceOf(address(this));
        uint profit = balanceAfter - balanceBefore;
        uint profitToCoinbase = (profit * percentageToPayToCoinbase) / 100;
        WETH.withdraw(profitToCoinbase);
        block.coinbase.transfer(profitToCoinbase);
        require(balanceAfter - profitToCoinbase > balanceBefore, "arb failed");

        uint256 gasUsed = gasStart - gasleft();
        uint256 profitAfterCoinbase = profit - profitToCoinbase;
        require(
            profitAfterCoinbase > gasUsed * getGasPrice(),
            "arb not profitable"
        );

        nextAddress = address(0);
        callBackAddress = address(0);
    }

    function getGasPrice() private view returns (uint256) {
        uint256 gasPrice;
        assembly {
            gasPrice := gasprice()
        }
        return gasPrice;
    }

    function executeArb__WETH_token1(
        address v2Pair,
        address v3Pair,
        uint256 amountIn,
        uint256 percentageToPayToCoinbase
    ) public onlyOwner {
        uint256 gasStart = gasleft();
        callBackAddress = v3Pair;
        nextAddress = v2Pair;
        token0 = false;

        uint256 balanceBefore = WETH.balanceOf(address(this));

        // Swap on V3
        IUniswapV3Pool(v3Pair).swap(
            // is this correct v2Pair address here?
            address(this),
            false,
            int256(amountIn),
            MAX_SQRT_RATIO - 1,
            ""
        );

        uint256 balanceAfter = WETH.balanceOf(address(this));
        uint profit = balanceAfter - balanceBefore;
        uint profitToCoinbase = (profit * percentageToPayToCoinbase) / 100;
        WETH.withdraw(profitToCoinbase);
        block.coinbase.transfer(profitToCoinbase);
        require(balanceAfter - profitToCoinbase > balanceBefore, "arb failed");

        uint256 gasUsed = gasStart - gasleft();
        uint256 profitAfterCoinbase = profit - profitToCoinbase;
        require(
            profitAfterCoinbase > gasUsed * getGasPrice(),
            "arb not profitable"
        );

        token0 = true;
        nextAddress = address(0);
        callBackAddress = address(0);
    }

    /// Pay back WETH
    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata _data
    ) external override {
        require(msg.sender == callBackAddress, "invalid sender");
        uint256 amountOwed;
        uint256 tokenOutExact;
        if (amount0Delta < 0) {
            amountOwed = uint256(amount1Delta);
            tokenOutExact = uint256(-amount0Delta);
        } else {
            amountOwed = uint256(amount0Delta);
            tokenOutExact = uint256(-amount1Delta);
        }

        // when running from execute_arb_weth_token_0, nextAddress is address(0)
        // and pair resolving error is occured
        IUniswapV2Pair v2Pair = IUniswapV2Pair(nextAddress);
        (uint256 v2Reserve0, uint256 v2Reserve1, ) = v2Pair.getReserves();

        if (token0) {
            uint256 v2AmountOut = getAmountOut(
                tokenOutExact,
                v2Reserve1,
                v2Reserve0
            );
            v2Pair.swap(v2AmountOut, 0, address(this), "");
        } else {
            uint256 v2AmountOut = getAmountOut(
                tokenOutExact,
                v2Reserve0,
                v2Reserve1
            );
            v2Pair.swap(0, v2AmountOut, address(this), "");
        }

        WETH.transfer(callBackAddress, amountOwed);
    }

    function uniswapV2Call(
        address sender,
        uint256 amount0,
        uint256 amount1,
        bytes calldata data
    ) external {
        address token0 = IUniswapV2Pair(msg.sender).token0(); // fetch the address of token0
        address token1 = IUniswapV2Pair(msg.sender).token1(); // fetch the address of token1
        require(
            msg.sender == factoryV2.getPair(token0, token1),
            "It was called by not a uniswap"
        ); // ensure that msg.sender is a V2 pair
        require(sender == address(this), "Griefing detected");
        // require(
        //     amount0 <= IERC20(token0).balanceOf(address(this)),
        //     "Invalid balance, flashswap unsucsessful"
        // );
        address collateralToken = token0;
        address borrowedToken = token0;
        uint256 repayAmount;
        if (amount0 == 0) {
            borrowedToken = token1;
            repayAmount = amount1;
        } else {
            collateralToken = token1;
            repayAmount = amount0;
        }
        uint256 returnSwapAmount = getTokenPrice(
            msg.sender,
            repayAmount,
            token0 == collateralToken
        );
        IERC20(collateralToken).transfer(msg.sender, returnSwapAmount);
    }

    function getAmountOut(
        uint256 amountIn,
        uint256 reserveIn,
        uint256 reserveOut
    ) internal pure returns (uint256 amountOut) {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        amountOut = numerator / denominator;
    }

    function getTokenPrice(
        address pairAddress,
        uint256 amount,
        bool return0
    ) internal view returns (uint256) {
        (uint256 Res0, uint256 Res1, ) = IUniswapV2Pair(pairAddress)
            .getReserves();

        uint256 reserveIn;
        uint256 reserveOut;
        if (return0) {
            reserveIn = Res0;
            reserveOut = Res1;
        } else {
            reserveIn = Res1;
            reserveOut = Res0;
        }
        uint256 numerator = reserveIn.mul(amount).mul(1000);

        uint256 denominator = reserveOut.sub(amount).mul(997);

        return (numerator / denominator).add(1);
    }

    function withdrawWETHToOwner() external onlyOwner {
        uint256 balance = WETH.balanceOf(address(this));
        WETH.transfer(msg.sender, balance);
    }

    function withdrawETHToOwner() external onlyOwner {
        uint256 balance = address(this).balance;
        payable(msg.sender).transfer(balance);
    }

    receive() external payable {}
}
