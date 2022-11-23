# Fraction

Fraction is a platform where users can invest in assets, with the help of fractional NFTs. We set the entry barrier for high value items very low and therefore allow a brought investor base to invest in an asset class, that wasn’t accessible for them beforehand.

 We research items with a high growth value and propose them on our website. Users can connect with their Radix wallet and buy fractions of the asset. One asset is represented by one NFT, which in turn is composed of multiple other NFTs. Those NFTs will verify the ownership of investors. Once a funding goal is reached, investors receive an NFT, representing their share and we will buy and store the item. When the asset increases in value according to our goals, we sell it and all holders share the profit. There is no risk if the funding goal is not reached, as users will get their money back immediately. 

## User Journey

User Connecting
<img src="https://github.com/J-Son89/scrypto-challenges/blob/main/6-nfts-for-financial-applications/fond/app/images/landingPage.png" alt="drawing" width="500"/>

The landing page, shows the user an overview of the website.

Users can connect their wallet using the AlphaNet wallet extension:
https://docs.radixdlt.com/main/scrypto/alphanet/wallet-extension.html


### Once connected

<img src="https://github.com/J-Son89/scrypto-challenges/blob/main/6-nfts-for-financial-applications/fond/app/images/investPage.png" alt="drawing" width="500"/>

A user can then go through the different campaigns of the project available on the 'Invest' tab.

This shows the user the target amount needed to raise for the project.
From there users can contribute any amount they wish.

After investing, users will then receive an NFT showing how much they have invested. 

### Getting funds back out

Once the company has fulfilled their target of funds to raise. The company will then buy the item and eventually sell it for profit.

After selling the item, the user can then retrieve their funds by hitting the Retrieve Funds button.

## Smart Contract
The smart contract for this application is in fond/src/lib.js

It splits the roles into two user types Admin & User.

### Admin
Admins can
- see available campaigns data
- create campaigns,
- buy items with completed campaigns
- sell items 

### User
Users can
- see available campaigns data
- invest in campaigns 
- retrieve funds from completed campaigns.
## Frontend
The frontend of this application is a basic React App.

Initially install the dependencies with `npm i` and then run `npm start` to run the application.

## What Next??
This project serves as an MVP for Fraction. Going forward Fraction has a wide set of ideas to implement.

### Validation
Given the easy nature of Scrypto smart contracts. We want to simplify the process of creating new admins and having admin protected functionality.

### Dao
Creating a campaign can be difficult when finding high value items with good potential returns. To improve this process, Fraction is considering creating a DAO where approved members can vote and contribute on the future investments.

### Improved User Journey
As the project grows, the requirement for Users to revoke their contribution from an investment will become more of a concern.
Further, if for some reason an investment seems to have lost it's potential, admins will have the functionality to cancel a campaign in which case contributors will be able to retrieve their funds.


