import { Component } from '@angular/core';
import { NavComponent } from '../nav/nav.component';

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [
    NavComponent
  ],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.css'
})
export class SidebarComponent {

    // Menu
    menu: boolean = false;
    showMenu() {
      console.log('menu');
      this.menu = !this.menu;
    }
}
