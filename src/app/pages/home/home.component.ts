import { Component } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { HeaderComponent } from '../../layout/header/header.component';
import { SidebarComponent } from '../../layout/sidebar/sidebar.component';
import { MainComponent } from '../../layout/main/main.component';
import { FooterComponent } from '../../layout/footer/footer.component';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
      RouterOutlet,
      HeaderComponent,
      SidebarComponent,
      MainComponent,
      FooterComponent
    ],
  templateUrl: './home.component.html',
  styleUrl: './home.component.css'
})
export default class HomeComponent {

}